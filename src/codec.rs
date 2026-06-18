//! The `.huff` container format plus `compress` / `decompress`.
//!
//! Layout:
//!   magic        "HUF1"            (4 bytes)
//!   original_len u64 little-endian (8 bytes)
//!   tree_nbits   u32 little-endian (4 bytes)
//!   tree         ceil(tree_nbits/8) bytes  (pre-order serialized Huffman tree)
//!   payload_pad  u8                (1 byte, padding bits in the last payload byte)
//!   payload      remaining bytes   (the Huffman bitstream)
//!
//! The tree is stored explicitly (not the frequencies) so the decoder rebuilds
//! the *exact* tree the encoder used — no tie-break ambiguity.

use crate::bitio::{BitReader, BitWriter};
use crate::huffman::{build_tree, freq_table, gen_codes, Node};

const MAGIC: &[u8; 4] = b"HUF1";

fn serialize_tree(node: &Node, w: &mut BitWriter) {
    match node {
        Node::Leaf { byte, .. } => {
            w.write_bit(true);
            for i in (0..8).rev() {
                w.write_bit((byte >> i) & 1 == 1);
            }
        }
        Node::Internal { left, right, .. } => {
            w.write_bit(false);
            serialize_tree(left, w);
            serialize_tree(right, w);
        }
    }
}

fn deserialize_tree(r: &mut BitReader) -> Result<Node, String> {
    match r.read_bit() {
        None => Err("unexpected end of tree data".into()),
        Some(true) => {
            let mut byte = 0u8;
            for _ in 0..8 {
                let bit = r.read_bit().ok_or("truncated leaf byte")?;
                byte = (byte << 1) | (bit as u8);
            }
            Ok(Node::Leaf { byte, freq: 0 })
        }
        Some(false) => {
            let left = deserialize_tree(r)?;
            let right = deserialize_tree(r)?;
            Ok(Node::Internal {
                freq: 0,
                left: Box::new(left),
                right: Box::new(right),
            })
        }
    }
}

pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());

    if data.is_empty() {
        out.extend_from_slice(&0u32.to_le_bytes()); // tree_nbits
        out.push(0); // payload_pad
        return out;
    }

    let freq = freq_table(data);
    let tree = build_tree(&freq).expect("non-empty input always has a tree");

    let mut tw = BitWriter::new();
    serialize_tree(&tree, &mut tw);
    let (tree_bytes, tree_pad) = tw.finish();
    let tree_nbits = (tree_bytes.len() * 8) as u32 - tree_pad as u32;
    out.extend_from_slice(&tree_nbits.to_le_bytes());
    out.extend_from_slice(&tree_bytes);

    let codes = gen_codes(&tree);
    let mut pw = BitWriter::new();
    for &b in data {
        pw.write_bits(&codes[&b]);
    }
    let (payload, pad) = pw.finish();
    out.push(pad);
    out.extend_from_slice(&payload);
    out
}

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 17 {
        return Err("file too short to be a .huff archive".into());
    }
    if &data[0..4] != MAGIC {
        return Err("bad magic — not a .huff file".into());
    }
    let original_len = u64::from_le_bytes(data[4..12].try_into().unwrap()) as usize;
    let tree_nbits = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let tree_bytes_len = tree_nbits.div_ceil(8);

    let mut pos = 16;
    if data.len() < pos + tree_bytes_len + 1 {
        return Err("truncated header".into());
    }
    let tree_bytes = &data[pos..pos + tree_bytes_len];
    pos += tree_bytes_len;
    let _payload_pad = data[pos];
    pos += 1;
    let payload = &data[pos..];

    if original_len == 0 {
        return Ok(Vec::new());
    }

    let mut tr = BitReader::new(tree_bytes);
    let tree = deserialize_tree(&mut tr)?;

    // Single distinct symbol: the whole file is one byte repeated.
    if let Node::Leaf { byte, .. } = tree {
        return Ok(vec![byte; original_len]);
    }

    let mut out = Vec::with_capacity(original_len);
    let mut pr = BitReader::new(payload);
    let mut node: &Node = &tree;
    while out.len() < original_len {
        match node {
            Node::Leaf { byte, .. } => {
                out.push(*byte);
                node = &tree;
            }
            Node::Internal { left, right, .. } => {
                let bit = pr.read_bit().ok_or("truncated payload")?;
                node = if bit { right.as_ref() } else { left.as_ref() };
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(data: &[u8]) {
        let packed = compress(data);
        let restored = decompress(&packed).expect("decompress ok");
        assert_eq!(restored, data, "round-trip mismatch for {} bytes", data.len());
    }

    #[test]
    fn empty() {
        roundtrip(b"");
    }

    #[test]
    fn single_byte_repeated() {
        roundtrip(b"zzzzzzzzzz");
    }

    #[test]
    fn english_text() {
        roundtrip(b"the quick brown fox jumps over the lazy dog, again and again.");
    }

    #[test]
    fn pseudo_random_4k() {
        // simple LCG so the test is deterministic without extra deps
        let mut x: u32 = 0x1234_5678;
        let mut data = Vec::with_capacity(4096);
        for _ in 0..4096 {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            data.push((x >> 16) as u8);
        }
        roundtrip(&data);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decompress(b"not a huff file at all").is_err());
    }
}

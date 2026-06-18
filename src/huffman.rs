//! Huffman tree construction and prefix-code generation.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

#[derive(Debug, Clone)]
pub enum Node {
    Leaf { byte: u8, freq: u64 },
    Internal { freq: u64, left: Box<Node>, right: Box<Node> },
}

impl Node {
    pub fn freq(&self) -> u64 {
        match self {
            Node::Leaf { freq, .. } => *freq,
            Node::Internal { freq, .. } => *freq,
        }
    }

    /// Smallest byte value under this node. Used purely as a deterministic
    /// tie-breaker so equal-frequency merges always happen in the same order —
    /// the resulting tree is canonical.
    fn min_byte(&self) -> u8 {
        match self {
            Node::Leaf { byte, .. } => *byte,
            Node::Internal { left, .. } => left.min_byte(),
        }
    }
}

/// Wrapper that makes the max-`BinaryHeap` behave as a min-heap keyed on
/// `(freq, min_byte)`.
struct HeapItem(Node);

impl PartialEq for HeapItem {
    fn eq(&self, o: &Self) -> bool {
        self.0.freq() == o.0.freq() && self.0.min_byte() == o.0.min_byte()
    }
}
impl Eq for HeapItem {}
impl Ord for HeapItem {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.freq()
            .cmp(&self.0.freq())
            .then_with(|| o.0.min_byte().cmp(&self.0.min_byte()))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

pub fn freq_table(data: &[u8]) -> [u64; 256] {
    let mut t = [0u64; 256];
    for &b in data {
        t[b as usize] += 1;
    }
    t
}

/// Build a Huffman tree from a frequency table. `None` for an all-zero table.
pub fn build_tree(freq: &[u64; 256]) -> Option<Node> {
    let mut heap = BinaryHeap::new();
    for (b, &f) in freq.iter().enumerate() {
        if f > 0 {
            heap.push(HeapItem(Node::Leaf { byte: b as u8, freq: f }));
        }
    }
    if heap.is_empty() {
        return None;
    }
    while heap.len() > 1 {
        let a = heap.pop().unwrap().0;
        let b = heap.pop().unwrap().0;
        let merged = Node::Internal {
            freq: a.freq() + b.freq(),
            left: Box::new(a),
            right: Box::new(b),
        };
        heap.push(HeapItem(merged));
    }
    Some(heap.pop().unwrap().0)
}

/// Map every leaf byte to its bit path (false = left/0, true = right/1).
pub fn gen_codes(root: &Node) -> HashMap<u8, Vec<bool>> {
    fn walk(n: &Node, path: &mut Vec<bool>, codes: &mut HashMap<u8, Vec<bool>>) {
        match n {
            Node::Leaf { byte, .. } => {
                // A tree of a single distinct symbol has a leaf at the root; give
                // it the 1-bit code "0" so it still encodes.
                let code = if path.is_empty() { vec![false] } else { path.clone() };
                codes.insert(*byte, code);
            }
            Node::Internal { left, right, .. } => {
                path.push(false);
                walk(left, path, codes);
                path.pop();
                path.push(true);
                walk(right, path, codes);
                path.pop();
            }
        }
    }
    let mut codes = HashMap::new();
    let mut path = Vec::new();
    walk(root, &mut path, &mut codes);
    codes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_and_complete() {
        let data = b"abracadabra";
        let freq = freq_table(data);
        let tree = build_tree(&freq).unwrap();
        let codes = gen_codes(&tree);
        // every distinct byte has a code
        for &b in data {
            assert!(codes.contains_key(&b));
        }
        // prefix-free: more frequent symbols get codes no longer than rarer ones
        assert!(codes[&b'a'].len() <= codes[&b'd'].len());
    }

    #[test]
    fn single_symbol() {
        let freq = freq_table(b"aaaa");
        let tree = build_tree(&freq).unwrap();
        let codes = gen_codes(&tree);
        assert_eq!(codes[&b'a'], vec![false]);
    }
}

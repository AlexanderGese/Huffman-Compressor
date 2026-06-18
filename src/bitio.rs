//! MSB-first bit I/O used by the encoder and the container format.

/// Packs individual bits into bytes, most-significant-bit first.
pub struct BitWriter {
    buf: Vec<u8>,
    cur: u8,
    nbits: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter { buf: Vec::new(), cur: 0, nbits: 0 }
    }

    pub fn write_bit(&mut self, bit: bool) {
        self.cur = (self.cur << 1) | (bit as u8);
        self.nbits += 1;
        if self.nbits == 8 {
            self.buf.push(self.cur);
            self.cur = 0;
            self.nbits = 0;
        }
    }

    pub fn write_bits(&mut self, bits: &[bool]) {
        for &b in bits {
            self.write_bit(b);
        }
    }

    /// Flush any partial byte (zero-padded) and return `(bytes, padding_bits)`.
    pub fn finish(mut self) -> (Vec<u8>, u8) {
        if self.nbits > 0 {
            let pad = 8 - self.nbits;
            self.cur <<= pad;
            self.buf.push(self.cur);
            (self.buf, pad)
        } else {
            (self.buf, 0)
        }
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Reads bits back out, most-significant-bit first.
pub struct BitReader<'a> {
    data: &'a [u8],
    byte: usize,
    bit: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BitReader { data, byte: 0, bit: 0 }
    }

    pub fn read_bit(&mut self) -> Option<bool> {
        if self.byte >= self.data.len() {
            return None;
        }
        let b = (self.data[self.byte] >> (7 - self.bit)) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.byte += 1;
        }
        Some(b == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_bits() {
        let bits = [true, false, true, true, false, false, false, true, true, false];
        let mut w = BitWriter::new();
        w.write_bits(&bits);
        let (bytes, _pad) = w.finish();
        let mut r = BitReader::new(&bytes);
        for &b in &bits {
            assert_eq!(r.read_bit(), Some(b));
        }
    }

    #[test]
    fn reader_runs_out() {
        let mut r = BitReader::new(&[0b1000_0000]);
        assert_eq!(r.read_bit(), Some(true));
        for _ in 0..7 {
            assert_eq!(r.read_bit(), Some(false));
        }
        assert_eq!(r.read_bit(), None);
    }
}

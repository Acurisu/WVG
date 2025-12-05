//! BitStream reader for parsing WVG binary data.
//!
//! Provides bit-level reading operations for the WVG format where data is packed
//! at the bit level rather than byte level.

use crate::error::{WvgError, WvgResult};

/// A bit-level stream reader for WVG binary data.
///
/// WVG uses MSB-first bit ordering within each byte. The bit position 0 corresponds
/// to the MSB (0x80), and bit position 7 corresponds to the LSB (0x01).
#[derive(Debug)]
pub struct BitStream<'a> {
    /// The underlying byte data
    data: &'a [u8],
    /// Current byte position
    byte_pos: usize,
    /// Current bit position within the byte (0 = MSB, 7 = LSB)
    bit_pos: u8,
}

impl<'a> BitStream<'a> {
    /// Creates a new BitStream from the given byte slice.
    ///
    /// # Arguments
    ///
    /// * `data` - The byte slice to read from
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    /// Reads a single bit from the stream.
    ///
    /// # Returns
    ///
    /// The bit value (0 or 1) or an error if end of stream is reached.
    ///
    /// # Errors
    ///
    /// Returns `WvgError::EndOfStream` if attempting to read past the end of data.
    pub fn read_bit(&mut self) -> WvgResult<u8> {
        if self.byte_pos >= self.data.len() {
            return Err(WvgError::EndOfStream);
        }

        let byte = self.data[self.byte_pos];
        // MSB is bit index 0, so we shift right by (7 - bit_pos)
        let bit = (byte >> (7 - self.bit_pos)) & 1;

        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }

        Ok(bit)
    }

    /// Reads `n` bits from the stream as an unsigned integer.
    ///
    /// Bits are read MSB-first and assembled into an integer value.
    ///
    /// # Arguments
    ///
    /// * `n` - The number of bits to read (1-32)
    ///
    /// # Returns
    ///
    /// The unsigned integer value formed by the read bits.
    ///
    /// # Errors
    ///
    /// Returns `WvgError::EndOfStream` if attempting to read past the end of data.    
    pub fn read_bits(&mut self, n: u8) -> WvgResult<u32> {
        let mut val: u32 = 0;
        for _ in 0..n {
            val = (val << 1) | (self.read_bit()? as u32);
        }
        Ok(val)
    }

    /// Reads `n` bits from the stream as a signed integer using two's complement.
    ///
    /// # Arguments
    ///
    /// * `n` - The number of bits to read (1-32)
    ///
    /// # Returns
    ///
    /// The signed integer value using two's complement representation.
    ///
    /// # Errors
    ///
    /// Returns `WvgError::EndOfStream` if attempting to read past the end of data.
    pub fn read_signed_bits(&mut self, n: u8) -> WvgResult<i32> {
        let val = self.read_bits(n)?;
        // Check if the sign bit (MSB of the n bits) is set
        if val & (1 << (n - 1)) != 0 {
            // Sign extend by subtracting 2^n
            Ok(val as i32 - (1 << n))
        } else {
            Ok(val as i32)
        }
    }

    /// Returns true if more bits are available.
    pub fn has_more_bits(&self) -> bool {
        self.byte_pos < self.data.len()
    }

    /// Returns the current byte position in the stream.
    pub fn byte_position(&self) -> usize {
        self.byte_pos
    }

    /// Returns the current bit position within the current byte.
    pub fn bit_position(&self) -> u8 {
        self.bit_pos
    }

    /// Returns the total number of bytes in the stream.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_single_bits() {
        let data = vec![0b10101010];
        let mut bs = BitStream::new(&data);

        assert_eq!(bs.read_bit().unwrap(), 1);
        assert_eq!(bs.read_bit().unwrap(), 0);
        assert_eq!(bs.read_bit().unwrap(), 1);
        assert_eq!(bs.read_bit().unwrap(), 0);
        assert_eq!(bs.read_bit().unwrap(), 1);
        assert_eq!(bs.read_bit().unwrap(), 0);
        assert_eq!(bs.read_bit().unwrap(), 1);
        assert_eq!(bs.read_bit().unwrap(), 0);
    }

    #[test]
    fn test_read_bits() {
        let data = vec![0b11110000, 0b00001111];
        let mut bs = BitStream::new(&data);

        assert_eq!(bs.read_bits(4).unwrap(), 0b1111);
        assert_eq!(bs.read_bits(8).unwrap(), 0b00000000);
        assert_eq!(bs.read_bits(4).unwrap(), 0b1111);
    }

    #[test]
    fn test_read_signed_bits_positive() {
        let data = vec![0b01111111]; // 127 in 8-bit signed
        let mut bs = BitStream::new(&data);

        assert_eq!(bs.read_signed_bits(8).unwrap(), 127);
    }

    #[test]
    fn test_read_signed_bits_negative() {
        let data = vec![0b11111111]; // -1 in 8-bit two's complement
        let mut bs = BitStream::new(&data);

        assert_eq!(bs.read_signed_bits(8).unwrap(), -1);
    }

    #[test]
    fn test_read_signed_bits_negative_small() {
        let data = vec![0b11100000]; // Reading 3 bits: 111 = -1
        let mut bs = BitStream::new(&data);

        assert_eq!(bs.read_signed_bits(3).unwrap(), -1);
    }

    #[test]
    fn test_end_of_stream() {
        let data = vec![0xFF];
        let mut bs = BitStream::new(&data);

        // Read all 8 bits
        for _ in 0..8 {
            bs.read_bit().unwrap();
        }

        // Next read should fail
        assert!(matches!(bs.read_bit(), Err(WvgError::EndOfStream)));
    }

    #[test]
    fn test_has_more_bits() {
        let data = vec![0xFF];
        let mut bs = BitStream::new(&data);

        assert!(bs.has_more_bits());

        // Read all bits
        for _ in 0..8 {
            bs.read_bit().unwrap();
        }

        assert!(!bs.has_more_bits());
    }

    #[test]
    fn test_cross_byte_boundary() {
        let data = vec![0b11110000, 0b11110000];
        let mut bs = BitStream::new(&data);

        // Read 6 bits from first byte
        assert_eq!(bs.read_bits(6).unwrap(), 0b111100);
        // Read 6 bits crossing byte boundary
        assert_eq!(bs.read_bits(6).unwrap(), 0b001111);
    }
}

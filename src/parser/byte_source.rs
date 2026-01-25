//! Byte source abstractions for parser.
//!
//! This module provides the [ByteSource] trait and implementations for different
//! ways of accessing byte data during parser. Supports both in-memory sources
//! ([InMemoryByteSource]) and potentially buffered file reading.

// =#========================================================================#=
// BYTE SOURCE (Trait)
// =#========================================================================#=
/// Trait defining the interface for different byte sources used by ByteParser.
///
/// This trait abstracts over different ways of accessing byte data:
/// - In-memory byte slices (`&[u8]`)
/// - Buffered reading from files (`BufReader<File>`)
///
/// By using this trait, the same parser logic can work with both small files
/// loaded entirely into memory and large files streamed from disk.
pub trait ByteSource {
    /// Peek at the current byte without consuming it.
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of data (EOF)
    fn peek(&self) -> Option<u8>;

    /// Get the current byte and advance the position (consume it).
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of data (EOF)
    fn next_byte(&mut self) -> Option<u8>;

    /// Returns the current position in the byte stream.
    ///
    /// # Returns
    /// The current byte offset
    fn position(&self) -> usize;

    /// Sets the position in the byte stream.
    ///
    /// # Arguments
    /// * `pos` - The byte offset to seek to
    fn set_position(&mut self, pos: usize);

    /// Returns a slice of bytes from a start position to the current position.
    ///
    /// # Arguments
    /// * `start` - The starting byte offset
    ///
    /// # Returns
    /// A byte slice from `start` to the current position, or `None` if not available
    fn slice_from(&self, start: usize) -> Option<&[u8]>;

    /// Returns up to `k` bytes from the current position for error context.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A vector containing up to `k` bytes (or fewer if EOF reached)
    fn get_context(&self, k: usize) -> Vec<u8>;

    /// Returns a slice of up to `k` bytes from the current position without allocating.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A byte slice containing up to `k` bytes (or fewer if EOF reached)
    fn peek_slice(&self, k: usize) -> &[u8];

    /// Check if at end of data.
    ///
    /// # Returns
    /// `true` if at or beyond the end of data, `false` otherwise
    fn is_eof(&self) -> bool;
}

/// An in-memory byte source that owns its data.
///
/// This is the most efficient byte source for files that can fit entirely in memory.
pub struct InMemoryByteSource {
    /// The owned byte data being parsed
    input: Vec<u8>,
    /// Current position in the byte slice
    pos: usize,
}

// =#========================================================================#=
// IN MEMORY BYTE SOURCE
// =#========================================================================#=
impl InMemoryByteSource {
    /// Creates a new in-memory byte source from a Vec of bytes.
    ///
    /// # Arguments
    /// * `bytes` - The byte vector to parse
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self {
            input: bytes,
            pos: 0,
        }
    }
}

impl ByteSource for InMemoryByteSource {
    #[inline(always)]
    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    #[inline(always)]
    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    #[inline]
    fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn slice_from(&self, start: usize) -> Option<&[u8]> {
        if start <= self.pos && self.pos <= self.input.len() {
            Some(&self.input[start..self.pos])
        } else {
            None
        }
    }

    fn get_context(&self, k: usize) -> Vec<u8> {
        let end = (self.pos + k).min(self.input.len());
        self.input[self.pos..end].to_vec()
    }

    #[inline(always)]
    fn peek_slice(&self, k: usize) -> &[u8] {
        let end = (self.pos + k).min(self.input.len());
        &self.input[self.pos..end]
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

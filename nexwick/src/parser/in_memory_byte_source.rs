//! In-memory implementation of byte source for parser.

use crate::parser::byte_source::ByteSource;
use std::fs::File;
use std::io::Read;
use std::path::Path;

// =#========================================================================#=
// IN MEMORY BYTE SOURCE
// =#========================================================================$=
/// An in-memory byte source that owns its data.
///
/// This is the most efficient byte source for files
/// that can fit entirely in memory.
pub struct InMemoryByteSource {
    /// The owned byte data being parsed
    input: Vec<u8>,
    /// Current position in the byte slice
    pos: usize,
}

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

    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<InMemoryByteSource> {
        // Read entire file into memory
        let mut contents = Vec::new();
        let mut file = File::open(path)?;
        file.read_to_end(&mut contents)?;
        Ok(Self {
            input: contents,
            pos: 0,
        })
    }
}

impl ByteSource for InMemoryByteSource {
    #[inline(always)]
    fn peek(&mut self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    #[inline(always)]
    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    #[inline(always)]
    fn peek_slice(&mut self, k: usize) -> &[u8] {
        let end = (self.pos + k).min(self.input.len());
        &self.input[self.pos..end]
    }

    fn get_context(&mut self, k: usize) -> Vec<u8> {
        let end = (self.pos + k).min(self.input.len());
        self.input[self.pos..end].to_vec()
    }

    #[inline]
    fn position(&self) -> usize {
        self.pos
    }

    #[inline]
    fn set_position(&mut self, pos: usize) {
        self.pos = pos;
    }

    fn is_eof(&mut self) -> bool {
        self.pos >= self.input.len()
    }
}

//! Byte source abstractions for parser.
//!
//! This module provides the [ByteSource] trait and implementations for different
//! ways of accessing byte data during parser.

// =#========================================================================#=
// BYTE SOURCE (Trait)
// =#========================================================================T=
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
    fn peek(&mut self) -> Option<u8>;

    /// Get the current byte and advance the position (consume it).
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of data (EOF)
    fn next_byte(&mut self) -> Option<u8>;

    /// Returns a slice of up to `k` bytes from the current position without allocating.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A byte slice containing up to `k` bytes (or fewer if EOF reached)
    fn peek_slice(&mut self, k: usize) -> &[u8];

    /// Returns up to `k` bytes from the current position for error context.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A vector containing up to `k` bytes (or fewer if EOF reached)
    fn get_context(&mut self, k: usize) -> Vec<u8>;

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

    /// Check if at end of data.
    ///
    /// # Returns
    /// `true` if at or beyond the end of data, `false` otherwise
    fn is_eof(&mut self) -> bool;
}

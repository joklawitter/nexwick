//! Low-level byte-by-byte parser for ASCII text.
//!
//! This module provides [ByteParser] for parser text-based file formats with support
//! for peeking, consuming, pattern matching, and quote-aware label parser. Used as
//! the foundation for both NEXUS and Newick parsers.

use crate::parser::byte_parser::ConsumeMode::Inclusive;
use crate::parser::byte_source::{ByteSource, InMemoryByteSource};
use crate::parser::parsing_error::ParsingError;

// =#========================================================================#=
// BYTE PARSER
// =#========================================================================#=
/// A byte-by-byte parser for ASCII text with support for peeking, consuming, and pattern matching.
///
/// [ByteParser] provides parser operations for text-based formats, specifically targeting Newick and NEXUS.
/// It operates on byte sources and assumes ASCII encoding, offering both peek, consume,
/// and skip operations with case-insensitive matching.
///
/// # Features
/// - Works with any ByteSource (in-memory or buffered)
/// - Case-insensitive matching for ASCII characters
/// - Whitespace and comment skipping
/// - Quote-aware label parser (single quotes with escaping)
/// - Context extraction for error reporting
///
/// # TODOs
/// - Make consume_until methods comment-sensitive
///
/// # Example
/// ```ignore
/// use nexus_parser::parser::byte_parser::ByteParser;
/// use nexus_parser::parser::byte_source::InMemoryByteSource;
///
/// let input = "BEGIN TREES;\n  TREE t1 = (A:1.0,B:1.0):0.0;";
/// let source = InMemoryByteSource::new(input.as_bytes());
/// let mut parser = ByteParser::new(source);
///
/// parser.skip_whitespace();
/// assert!(parser.peek_is_word("BEGIN"));
/// parser.consume_if_word("BEGIN");
/// parser.skip_whitespace();
/// assert!(parser.peek_is_word("TREES"));
/// ```
pub struct ByteParser<S: ByteSource> {
    source: S,
}

impl ByteParser<InMemoryByteSource> {
    /// Creates a new `ByteParser` from a byte slice by copying it into a Vec.
    ///
    /// # Arguments
    /// * `input` - The byte slice to parse
    pub fn from_bytes(input: &[u8]) -> Self {
        Self::new(InMemoryByteSource::from_vec(input.to_vec()))
    }

    /// Creates a new `ByteParser` from a String by copying it into a Vec.
    ///
    /// # Arguments
    /// * `input` - The string to parse
    pub fn from_str(input: &str) -> Self {
        Self::new(InMemoryByteSource::from_vec(input.as_bytes().to_vec()))
    }
}

impl<S: ByteSource> ByteParser<S> {
    /// Creates a new `ByteParser` from a byte source.
    ///
    /// # Arguments
    /// * `source` - The byte source to parse
    pub fn new(source: S) -> Self {
        Self { source }
    }

    /// Peeks at the current byte without consuming it.
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of data (EOF)
    #[inline(always)]
    pub fn peek(&self) -> Option<u8> {
        self.source.peek()
    }

    /// Gets the current byte and advances the position (consumes it).
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of data (EOF)
    #[inline(always)]
    pub fn next(&mut self) -> Option<u8> {
        self.source.next()
    }

    /// Skips (consumes) all consecutive whitespace characters.
    ///
    /// Whitespace includes: space (' '), tab ('\t'), newline ('\n'), and carriage return ('\r').
    pub fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.next();
            } else {
                break;
            }
        }
    }

    /// Skips (consumes) a NEXUS-style comment if present.
    ///
    /// NEXUS comments are enclosed in square brackets `[...]`.
    ///
    /// # Returns
    /// * `Ok(true)` - A comment was found and consumed
    /// * `Ok(false)` - No comment at current position
    /// * `Err(ParsingError)` - Comment was opened but never closed
    ///
    /// # Errors
    /// Returns an error if a comment starts with `[` but doesn't have a closing `]`.
    pub fn skip_comment(&mut self) -> Result<bool, ParsingError> {
        if self.consume_if(b'[') {
            if !self.consume_until(b']', Inclusive) {
                return Err(ParsingError::unclosed_comment(self));
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Skips (consumes) all consecutive whitespace and NEXUS comments.
    ///
    /// This method repeatedly skips whitespace and comments until no more are found.
    /// Useful for advancing to the next meaningful token in NEXUS files.
    ///
    /// # Errors
    /// Returns an error if an unclosed comment is encountered.
    pub fn skip_comment_and_whitespace(&mut self) -> Result<(), ParsingError> {
        self.skip_whitespace();

        while self.skip_comment()? {
            self.skip_whitespace();
        }

        Ok(())
    }

    /// Checks if the current byte matches the target byte (case-insensitive for ASCII).
    ///
    /// # Arguments
    /// * `ch` - The byte to match against
    ///
    /// # Returns
    /// `true` if the current byte matches `ch` in any case, `false` otherwise
    pub fn peek_is(&self, ch: u8) -> bool {
        self.peek() == Some(ch) ||
            self.peek() == Some(ch.to_ascii_lowercase()) ||
            self.peek() == Some(ch.to_ascii_uppercase())
    }

    /// Checks if the following bytes match the given word/token (case-insensitive).
    ///
    /// This is a peek operation - the parser position is not changed.
    ///
    /// # Arguments
    /// * `word` - The string to match against
    ///
    /// # Returns
    /// `true` if the next bytes match `word` (case-insensitive), `false` otherwise
    pub fn peek_is_word(&self, word: &str) -> bool {
        self.peek_is_sequence(word.as_bytes())
    }

    /// Checks if the following bytes match the given byte sequence (case-insensitive).
    ///
    /// This is a peek operation - not fully optimized for all ByteSources.
    ///
    /// # Arguments
    /// * `sequence` - The byte sequence to match against
    ///
    /// # Returns
    /// `true` if the next bytes match `sequence` (case-insensitive), `false` otherwise
    #[inline]
    pub fn peek_is_sequence(&self, sequence: &[u8]) -> bool {
        // Get slice without allocating
        let context = self.source.peek_slice(sequence.len());

        if context.len() < sequence.len() {
            return false;
        }

        // Case-insensitive comparison
        for (context_byte, seq_byte) in context.iter().zip(sequence.iter()) {
            if *context_byte != *seq_byte
                && *context_byte != seq_byte.to_ascii_lowercase()
                && *context_byte != seq_byte.to_ascii_uppercase() {
                return false;
            }
        }

        true
    }

    /// Consumes the current byte if it matches the target byte (case-insensitive).
    ///
    /// # Arguments
    /// * `ch` - The byte to match and consume
    ///
    /// # Returns
    /// `true` if the byte was matched and consumed, `false` otherwise
    pub fn consume_if(&mut self, ch: u8) -> bool {
        if self.peek_is(ch) {
            self.next();
            true
        } else {
            false
        }
    }

    /// Consumes the next bytes if they match the given word/token (case-insensitive).
    ///
    /// # Arguments
    /// * `word` - The string to match and consume
    ///
    /// # Returns
    /// `true` if the word was matched and consumed, `false` otherwise
    pub fn consume_if_word(&mut self, word: &str) -> bool {
        let word_bytes = word.as_bytes();
        self.consume_if_sequence(word_bytes)
    }

    /// Consumes the next bytes if they match the given byte sequence (case-insensitive).
    ///
    /// # Arguments
    /// * `sequence` - The byte sequence to match and consume
    ///
    /// # Returns
    /// `true` if the sequence was matched and consumed, `false` otherwise
    pub fn consume_if_sequence(&mut self, sequence: &[u8]) -> bool {
        // Check if the sequence matches
        if !self.peek_is_sequence(sequence) {
            return false;
        }

        // Consume the bytes
        for _ in 0..sequence.len() {
            self.next();
        }

        true
    }

    /// Consumes bytes until the target byte is found.
    ///
    /// # Arguments
    /// * `target` - The byte to search for
    /// * `mode` - Whether to consume the target byte (`Inclusive`) or stop before it (`Exclusive`)
    ///
    /// # Returns
    /// `true` if the target was found, `false` if EOF was reached first
    pub fn consume_until(&mut self, target: u8, mode: ConsumeMode) -> bool {
        while let Some(b) = self.peek() {
            if b == target {
                if mode == ConsumeMode::Inclusive {
                    self.next();
                }
                return true;
            }
            self.next();
        }
        false // reached EOF without finding target
    }

    /// Consumes bytes until any of the target bytes is found.
    ///
    /// # Arguments
    /// * `targets` - The set of bytes to search for
    /// * `mode` - Whether to consume the found byte (`Inclusive`) or stop before it (`Exclusive`)
    ///
    /// # Returns
    /// `Some(u8)` with the found byte, or `None` if EOF was reached first
    pub fn consume_until_any(&mut self, targets: &[u8], mode: ConsumeMode) -> Option<u8> {
        while let Some(b) = self.peek() {
            if targets.contains(&b) {
                if mode == ConsumeMode::Inclusive {
                    self.next();
                }
                return Some(b); // return which one we found
            }

            self.next();
        }
        None // reached EOF without finding target
    }

    /// Consumes bytes until the next bytes match the given word/token (case-insensitive).
    ///
    /// # Arguments
    /// * `word` - The string to search for
    /// * `mode` - Whether to consume the word (`Inclusive`) or stop before it (`Exclusive`)
    ///
    /// # Returns
    /// `true` if the word was found, `false` if EOF was reached first
    pub fn consume_until_word(&mut self, word: &str, mode: ConsumeMode) -> bool {
        self.consume_until_sequence(word.as_bytes(), mode)
    }

    /// Consumes bytes until the next bytes match the given byte sequence (case-insensitive).
    ///
    /// # Arguments
    /// * `sequence` - The byte sequence to search for
    /// * `mode` - Whether to consume the sequence (`Inclusive`) or stop before it (`Exclusive`)
    ///
    /// # Returns
    /// `true` if the sequence was found, `false` if EOF was reached first
    pub fn consume_until_sequence(&mut self, sequence: &[u8], mode: ConsumeMode) -> bool {
        loop {
            if self.is_eof() {
                return false;
            }

            // Check if we match the sequence at current position
            if self.peek_is_sequence(sequence) {
                if mode == ConsumeMode::Inclusive {
                    // Consume the sequence
                    for _ in 0..sequence.len() {
                        self.next();
                    }
                }
                return true;
            }

            // Move forward one byte
            self.next();
        }
    }

    /// Returns whether the end of data (EOF) has been reached.
    ///
    /// # Returns
    /// `true` if at or beyond the end of data, `false` otherwise
    pub fn is_eof(&self) -> bool {
        self.source.is_eof()
    }

    /// Returns the current parser position in the input.
    ///
    /// Useful for error messages and tracking parser state.
    ///
    /// # Returns
    /// The current byte offset in the input
    pub fn position(&self) -> usize {
        self.source.position()
    }

    /// Sets the position in the byte stream.
    ///
    /// # Arguments
    /// * `pos` - The byte offset to seek to
    pub fn set_position(&mut self, pos: usize) {
        self.source.set_position(pos);
    }

    /// Returns a slice of the input from a start position to the current position.
    ///
    /// # Arguments
    /// * `start` - The starting byte offset
    ///
    /// # Returns
    /// A byte slice from `start` to the current position, or empty slice if not available
    pub fn slice_from(&self, start: usize) -> &[u8] {
        self.source.slice_from(start).unwrap_or(&[])
    }

    /// Returns up to `k` bytes from the current position for error context.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A vector containing up to `k` bytes (or fewer if EOF reached)
    pub fn get_context(&self, k: usize) -> Vec<u8> {
        self.source.get_context(k)
    }

    /// Returns a string from up to `k` bytes from the current position for error context.
    ///
    /// Invalid UTF-8 sequences are replaced with the Unicode replacement character.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A string containing up to `k` bytes (or fewer if EOF reached)
    pub fn get_context_as_string(&self, k: usize) -> String {
        let context_bytes = &self.get_context(k);
        String::from_utf8_lossy(context_bytes).chars().collect()
    }

    /// Parses a label (quoted or unquoted) with the given delimiter set.
    ///
    /// This method automatically detects whether the label is quoted (single quotes)
    /// or unquoted and calls the appropriate parser method.
    ///
    /// # Arguments
    /// * `delimiters` - Byte array of characters that end an unquoted label
    ///
    /// # Returns
    /// The parsed label string
    ///
    /// # Errors
    /// Returns an error if quote parser fails
    pub fn parse_label(&mut self, delimiters: &[u8]) -> Result<String, ParsingError> {
        self.skip_comment_and_whitespace()?;

        if self.peek() == Some(b'\'') {
            self.parse_quoted_label()
        } else {
            self.parse_unquoted_label(delimiters)
        }
    }

    /// Parses a quoted label enclosed in single quotes with escape support.
    ///
    /// Assumes the opening quote has not been consumed yet. Single quotes within
    /// the label are escaped by doubling them (e.g., `'Wilson''s'` becomes `Wilson's`).
    ///
    /// # Returns
    /// The parsed label string without the enclosing quotes
    ///
    /// # Errors
    /// Returns an error if the quoted label is not properly closed
    pub fn parse_quoted_label(&mut self) -> Result<String, ParsingError> {
        self.next(); // consume opening '

        let mut label = String::new();
        while let Some(b) = self.next() {
            if b == b'\'' {
                // Check for escaped quote (two single quotes in a row)
                if self.peek() == Some(b'\'') {
                    label.push('\'');
                    self.next(); // consume second quote
                } else {
                    // End of quoted label
                    break;
                }
            } else {
                label.push(b as char);
            }
        }

        Ok(label)
    }

    /// Parses an unquoted label until any of the given delimiters is encountered.
    ///
    /// # Arguments
    /// * `delimiters` - Byte array of characters that terminate the label
    ///
    /// # Returns
    /// The parsed label string
    ///
    /// # Errors
    /// Currently does not return errors, but returns `Result` for API consistency
    pub fn parse_unquoted_label(&mut self, delimiters: &[u8]) -> Result<String, ParsingError> {
        let mut label = String::new();

        while let Some(b) = self.peek() {
            // Stop at any delimiter
            if delimiters.contains(&b) {
                break;
            }
            label.push(b as char);
            self.next();
        }

        Ok(label)
    }
}

/// Specifies whether to consume or leave the target when using `consume_until` methods.
///
/// This enum controls the behavior of various `consume_until` methods in `ByteParser`,
/// determining whether the target byte/sequence should be consumed along with everything
/// before it, or whether the parser should stop just before the target.
///
/// # Examples
/// ```
/// use nexwick::parser::parser::byte_parser::{ByteParser, ConsumeMode};
///
/// let mut parser = ByteParser::from_str("TREE t1=((A:0.5,B:0.5):0.3,C:0.8):0.0");
///
/// // Inclusive: consume up to and including '=', e.g. to start of Newick string
/// parser.consume_until(b'=', ConsumeMode::Inclusive);
/// assert_eq!(parser.peek(), Some(b'(')); // positioned after '='
///
/// let mut parser = ByteParser::from_str("('Wilson''s_Storm-petrel')");
///
/// // Exclusive: consume up to but not including "'", e.g. quoted comment start
/// parser.consume_until(b'\'', ConsumeMode::Exclusive);
/// assert_eq!(parser.peek(), Some(b'\'')); // positioned at '\''
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ConsumeMode {
    /// Consume the target byte/sequence along with everything before it.
    ///
    /// When using `Inclusive` mode, the parser position will be advanced past the target.
    Inclusive,

    /// Stop before the target byte/sequence without consuming it.
    ///
    /// When using `Exclusive` mode, the parser position will be at the target.
    Exclusive,
}

#[cfg(test)]
mod tests {}
use crate::parser::byte_parser::ConsumeMode::Inclusive;
use crate::parser::parsing_error::ParsingError;

/// A byte-by-byte parser for ASCII text with support for peeking, consuming, and pattern matching.
///
/// [ByteParser] provides parsing operations for text-based formats, specifically targeting Newick and NEXUS.
/// It operates on byte slices and assumes ASCII encoding, offering both peek, consume,
/// and skip operations with case-insensitive matching.
///
/// # Features
/// - Zero-copy parsing using byte slices
/// - Case-insensitive matching for ASCII characters
/// - Whitespace and comment skipping
/// - Quote-aware label parsing (single quotes with escaping)
/// - Context extraction for error reporting
///
/// # TODOs
/// - Make consume_until methods comment-sensitive
///
/// # Example
/// ```
/// use nexus_parser::parser::byte_parser::ByteParser;
///
/// let input = "BEGIN TREES;\n  TREE t1 = (A:1.0,B:1.0):0.0;";
/// let mut parser = ByteParser::from_str(input);
///
/// parser.skip_whitespace();
/// assert!(parser.peek_is_word("BEGIN"));
/// parser.consume_if_word("BEGIN");
/// parser.skip_whitespace();
/// assert!(parser.peek_is_word("TREES"));
/// ```
pub struct ByteParser<'a> {
    /// Byte slice being parsed
    input: &'a [u8],
    /// Current position of parser
    pos: usize,
}

impl<'a> ByteParser<'a> {
    /// Creates a new `ByteParser` from a string slice.
    ///
    /// # Arguments
    /// * `slice` - The string slice to parse
    ///
    /// # Example
    /// ```
    /// use nexus_parser::parser::byte_parser::ByteParser;
    ///
    /// let text = "TREE t1 = (A,B);";
    /// let mut parser = ByteParser::from_str(text);
    /// assert!(parser.peek_is_word("tree")); // case insensitive
    /// ```
    pub fn from_str(slice: &'a str) -> Self {
        ByteParser::from_bytes(slice.as_bytes())
    }

    /// Creates a new `ByteParser` from a byte slice.
    ///
    /// # Arguments
    /// * `bytes` - The byte slice to parse
    ///
    /// # Example
    /// ```
    /// use nexus_parser::parser::byte_parser::ByteParser;
    ///
    /// let data = b"TREE t1 = (A,B);";
    /// let mut parser = ByteParser::from_bytes(data);
    /// assert!(parser.peek_is_word("TREE"));
    /// ```
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            input: bytes,
            pos: 0,
        }
    }

    /// Peeks at the current byte without consuming it.
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of slice (EOF)
    pub fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    /// Gets the current byte and advances the position (consumes it).
    ///
    /// # Returns
    /// * `Some(u8)` - The current byte if available
    /// * `None` - If at end of slice (EOF)
    pub fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
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
    pub fn peek_is_word(&mut self, word: &str) -> bool {
        self.peek_is_sequence(word.as_bytes())
    }

    /// Checks if the following bytes match the given byte sequence (case-insensitive).
    ///
    /// This is a peek operation - the parser position is not changed.
    ///
    /// # Arguments
    /// * `sequence` - The byte sequence to match against
    ///
    /// # Returns
    /// `true` if the next bytes match `sequence` (case-insensitive), `false` otherwise
    pub fn peek_is_sequence(&mut self, sequence: &[u8]) -> bool {
        let original_position = self.pos;

        for seq_byte in sequence {
            let input_byte = match self.next() {
                Some(b) => b,
                None => {
                    // reached EOF before finding sequence
                    self.pos = original_position;
                    return false;
                }
            };

            if input_byte != *seq_byte
                && input_byte != seq_byte.to_ascii_lowercase()
                && input_byte != seq_byte.to_ascii_uppercase() {
                // found byte mismatch
                self.pos = original_position;
                return false;
            }
        }

        // Restore position since we only peek
        self.pos = original_position;
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
        let seq_len = sequence.len();

        // Check if we have enough bytes remaining
        if self.pos + seq_len > self.input.len() {
            return false;
        }

        // Check if the next bytes match the word (case-insensitive)
        for i in 0..seq_len {
            let input_byte = self.input[self.pos + i];
            let seq_byte = sequence[i];

            // Compare case-insensitively
            if input_byte != seq_byte
                && input_byte != seq_byte.to_ascii_lowercase()
                && input_byte != seq_byte.to_ascii_uppercase() {
                return false;
            }
        }

        // Match found, consume it
        self.pos += seq_len;
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
                    self.pos += 1;
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
                    self.pos += 1;
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
        let seq_len = sequence.len();

        while self.pos + seq_len <= self.input.len() {
            let mut matches = true;
            for i in 0..seq_len {
                let input_byte = self.input[self.pos + i];
                let seq_byte = sequence[i];

                if input_byte != seq_byte
                    && input_byte != seq_byte.to_ascii_lowercase()
                    && input_byte != seq_byte.to_ascii_uppercase() {
                    matches = false;
                    break;
                }
            }


            if matches {
                if mode == ConsumeMode::Inclusive {
                    self.pos += seq_len;
                }
                return true;
            }

            self.pos += 1;
        }

        false
    }

    /// Returns whether the end of slice (EOF) has been reached.
    ///
    /// # Returns
    /// `true` if at or beyond the end of slice, `false` otherwise
    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Returns the current parser position in the input.
    ///
    /// Useful for error messages and tracking parser state.
    ///
    /// # Returns
    /// The current byte offset in the input
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Returns a slice of the input from a start position to the current position.
    ///
    /// # Arguments
    /// * `start` - The starting byte offset
    ///
    /// # Returns
    /// A byte slice from `start` to the current position
    pub fn slice_from(&self, start: usize) -> &[u8] {
        &self.input[start..self.pos]
    }

    /// Returns up to `k` bytes from the current position for error context.
    ///
    /// # Arguments
    /// * `k` - Maximum number of bytes to retrieve
    ///
    /// # Returns
    /// A vector containing up to `k` bytes (or fewer if EOF reached)
    pub fn get_context(&self, k: usize) -> Vec<u8> {
        let end = (self.pos + k).min(self.input.len());
        self.input[self.pos..end].to_vec()
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
    /// or unquoted and calls the appropriate parsing method.
    ///
    /// # Arguments
    /// * `delimiters` - Byte array of characters that end an unquoted label
    ///
    /// # Returns
    /// The parsed label string
    ///
    /// # Errors
    /// Returns an error if quote parsing fails
    pub fn parse_label(&mut self, delimiters: &[u8]) -> Result<String, ParsingError> {
        self.skip_whitespace();

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
/// use nexus_parser::parser::byte_parser::{ByteParser, ConsumeMode};
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
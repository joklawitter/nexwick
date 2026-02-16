//! Buffered reader implementation of byte source for parser.
//!
//! This module provides [BufferedByteSource], which wraps a file in a [BufReader]
//! for efficient streaming I/O. Use this for large files where loading everything
//! into memory would be impractical.
//!

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::parser::byte_source::ByteSource;

// =#========================================================================#=
// BUFFERED BYTE SOURCE
// =#========================================================================$=
/// A buffered byte source for streaming large files.
///
/// Uses [BufReader] for efficient disk I/O and maintains its own buffer
/// to support peeking larger chunks.
///
/// # Implementation Note
/// Currently uses [BufReader] from std. In the future, we may implement our own
/// buffer to have more control over peek/backtrack operations without seeking.
pub struct BufferedByteSource {
    /// Underlying reader of file, handles getting chunks from file
    reader: BufReader<File>,

    /// Own buffer holding data read from the reader
    peek_buffer: Vec<u8>,

    /// Current absolute position in the stream
    pos: usize,
}

impl BufferedByteSource {
    /// Default capacity for the peek buffer.
    ///
    /// Sized to accommodate typical peek operations during parsing, such as
    /// checking for keywords like `#NEXUS` (6 bytes) or `TRANSLATE` (9 bytes).
    const PEEK_BUFFER_CAPACITY: usize = 16;

    /// Creates a new buffered byte source from a file path.
    ///
    /// # Arguments
    /// * `path` - Path to the file (accepting `&str`, `String`, `Path`, or `PathBuf`)
    ///
    /// # Errors
    /// Returns an error if the file cannot be opened.
    pub fn from_file<P: AsRef<Path>>(path: P) -> std::io::Result<BufferedByteSource> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self {
            reader,
            peek_buffer: Vec::with_capacity(Self::PEEK_BUFFER_CAPACITY),
            pos: 0,
        })
    }
}

impl ByteSource for BufferedByteSource {
    fn peek(&mut self) -> Option<u8> {
        let buf = self.reader.fill_buf().ok()?;
        buf.first().copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.reader.consume(1);
        self.pos += 1;
        Some(byte)
    }

    fn peek_slice(&mut self, k: usize) -> &[u8] {
        self.peek_buffer.clear();

        let buf = match self.reader.fill_buf() {
            Ok(b) => b,
            Err(_) => return &self.peek_buffer,
        };

        if buf.len() >= k {
            // Common case: enough data, just copy
            self.peek_buffer.extend_from_slice(&buf[..k]);
        } else {
            // Rare case: need more than available
            // Copy what we have, then consume and read more
            self.peek_buffer.extend_from_slice(buf);
            let mut consumed = buf.len();
            self.reader.consume(consumed);

            while self.peek_buffer.len() < k {
                let buf = match self.reader.fill_buf() {
                    Ok([]) => break, // EOF
                    Ok(b) => b,
                    Err(_) => break,
                };
                let need = k - self.peek_buffer.len();
                let take = need.min(buf.len());
                self.peek_buffer.extend_from_slice(&buf[..take]);
                self.reader.consume(take);
                consumed += take;
            }

            // Seek back to original position
            use std::io::{Seek, SeekFrom};
            let _ = self.reader.seek(SeekFrom::Current(-(consumed as i64)));
        }

        &self.peek_buffer
    }

    fn get_context(&mut self, k: usize) -> Vec<u8> {
        self.peek_slice(k).to_vec()
    }

    fn position(&self) -> usize {
        self.pos
    }

    fn set_position(&mut self, pos: usize) {
        use std::io::{Seek, SeekFrom};
        let _ = self.reader.seek(SeekFrom::Start(pos as u64));
        self.pos = pos;
    }

    fn is_eof(&mut self) -> bool {
        match self.reader.fill_buf() {
            Ok(buf) => buf.is_empty(),
            Err(_) => true,
        }
    }
}

// =#========================================================================#=
// TESTS - BUFFERED BYTE SOURCE
// =#========================================================================$=
#[cfg(test)]
mod tests {
    use crate::newick::NewickParser;
    use crate::parser::buffered_byte_source::BufferedByteSource;
    use crate::parser::byte_parser::ByteParser;

    #[test]
    fn test_buffered_parse_newick_file() {
        let source = BufferedByteSource::from_file("tests/fixtures/newick_t3_n10.nwk").unwrap();
        let byte_parser = ByteParser::new(source);
        let mut newick_parser = NewickParser::new_compact_defaults();

        let trees = newick_parser.parse_all(byte_parser).unwrap();
        assert_eq!(trees.len(), 3);
    }
}

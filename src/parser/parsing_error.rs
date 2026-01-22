//! Error types for NEXUS and Newick parser.
//!
//! This module provides [ParsingError] and [ParsingErrorType] for representing
//! and reporting errors that occur during parser of phylogenetic tree files.

use crate::parser::byte_parser::ByteParser;
use crate::parser::byte_source::ByteSource;
use std::error::Error;
use std::fmt;

/// Default length of context provided by error from parser
const DEFAULT_CONTEXT_LENGTH: usize = 50;


// =#========================================================================#=
// PARSING ERROR TYPE
// =#========================================================================€=
/// Error types that can occur during NEXUS and NEWICK parser.
#[derive(PartialEq, Debug, Clone)]
pub enum ParsingErrorType {
    IoError(String),
    UnexpectedEOF,
    MissingNexusHeader,
    InvalidBlockName,
    InvalidTaxaBlock(String),
    InvalidTreesBlock(String),
    InvalidTranslateCommand,
    UnclosedComment,
    InvalidNewickString(String),
    InvalidFormatting,
    UnresolvedLabel(String),
    BuilderNotInitialized,
    InvalidTreeStructure,
}


// =#========================================================================#=
// PARSING ERROR
// =#========================================================================$=
/// Parsing error with contextual information (position and surrounding bytes).
#[derive(Debug)]
pub struct ParsingError {
    kind: ParsingErrorType,
    position: usize,
    context: String,
}

impl ParsingError {
    /// Create a ParsingError from an error type and parser state
    pub fn from_parser<S: ByteSource>(kind: ParsingErrorType, parser: &ByteParser<S>) -> Self {
        Self {
            kind,
            position: parser.position(),
            context: parser.get_context_as_string(DEFAULT_CONTEXT_LENGTH),
        }
    }

    /// Convenience constructor for UnexpectedEOF
    pub fn unexpected_eof<S: ByteSource>(parser: &ByteParser<S>) -> Self {
        Self::from_parser(ParsingErrorType::UnexpectedEOF, parser)
    }

    /// Convenience constructor for MissingNexusHeader
    pub fn missing_nexus_header<S: ByteSource>(parser: &ByteParser<S>) -> Self {
        Self::from_parser(ParsingErrorType::MissingNexusHeader, parser)
    }

    /// Convenience constructor for InvalidBlockName
    pub fn invalid_block_name<S: ByteSource>(parser: &ByteParser<S>) -> Self {
        Self::from_parser(ParsingErrorType::InvalidBlockName, parser)
    }

    /// Convenience constructor for InvalidTaxaBlock
    pub fn invalid_taxa_block<S: ByteSource>(parser: &ByteParser<S>, msg: String) -> Self {
        Self::from_parser(ParsingErrorType::InvalidTaxaBlock(msg), parser)
    }

    /// Convenience constructor for InvalidTreesBlock
    pub fn invalid_trees_block<S: ByteSource>(parser: &ByteParser<S>, msg: String) -> Self {
        Self::from_parser(ParsingErrorType::InvalidTreesBlock(msg), parser)
    }

    /// Convenience constructor for InvalidTranslateCommand
    pub fn invalid_translate_command<S: ByteSource>(parser: &ByteParser<S>) -> Self {
        Self::from_parser(ParsingErrorType::InvalidTranslateCommand, parser)
    }

    /// Convenience constructor for UnclosedComment
    pub fn unclosed_comment<S: ByteSource>(parser: &ByteParser<S>) -> Self {
        Self::from_parser(ParsingErrorType::UnclosedComment, parser)
    }

    /// Convenience constructor for InvalidNewickString
    pub fn invalid_newick_string<S: ByteSource>(parser: &ByteParser<S>, msg: String) -> Self {
        Self::from_parser(ParsingErrorType::InvalidNewickString(msg), parser)
    }

    /// Convenience constructor for InvalidFormatting
    pub fn invalid_formatting<S: ByteSource>(parser: &ByteParser<S>) -> Self {
        Self::from_parser(ParsingErrorType::InvalidFormatting, parser)
    }

    /// Convenience constructor for Other error during parser
    pub fn unresolved_label<S: ByteSource>(parser: &ByteParser<S>, msg: String) -> Self {
        Self::from_parser(ParsingErrorType::UnresolvedLabel(msg), parser)
    }

    /// Create a ParsingError without parser context (for builder errors)
    pub fn without_context(kind: ParsingErrorType) -> Self {
        Self { kind, position: 0, context: String::new() }
    }

    /// Get the error kind
    pub fn kind(&self) -> &ParsingErrorType {
        &self.kind
    }

    /// Get the position where the error occurred
    pub fn position(&self) -> usize {
        self.position
    }
}

impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Main error message
        match &self.kind {
            ParsingErrorType::MissingNexusHeader => write!(f, "File does not start with #NEXUS header")?,
            ParsingErrorType::InvalidTaxaBlock(msg) => write!(f, "Invalid TAXA block format - {msg}")?,
            ParsingErrorType::InvalidTreesBlock(msg) => write!(f, "Invalid TREES block format - {msg}")?,
            ParsingErrorType::InvalidTranslateCommand => write!(f, "Invalid TRANSLATE command - likely inconsistent with TAXA block")?,
            ParsingErrorType::UnclosedComment => write!(f, "Unclosed comment")?,
            ParsingErrorType::InvalidBlockName => write!(f, "Invalid block name")?,
            ParsingErrorType::InvalidNewickString(msg) => write!(f, "Invalid newick string: {}", msg)?,
            ParsingErrorType::UnexpectedEOF => write!(f, "Unexpected end of file")?,
            ParsingErrorType::InvalidFormatting => write!(f, "Invalid formatting")?,
            ParsingErrorType::UnresolvedLabel(msg) => write!(f, "Could not resolve label - {msg}")?,
            ParsingErrorType::BuilderNotInitialized => write!(f, "Builder not initialized")?,
            ParsingErrorType::InvalidTreeStructure => write!(f, "Invalid tree structure")?,
            ParsingErrorType::IoError(msg) => write!(f, "IO error - {msg}")?,
        }

        // Additional position information
        write!(f, " at position {}", self.position)?;

        // Additional context if available
        if !self.context.is_empty() {
            write!(f, "\n  Context (next {} bytes): {}", self.context.len(), self.context)?;
        }

        Ok(())
    }
}

impl Error for ParsingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl From<std::io::Error> for ParsingError {
    fn from(err: std::io::Error) -> Self {
        ParsingError {
            kind: ParsingErrorType::IoError(err.to_string()),
            position: 0,  // No position for IO errors
            context: String::new(),  // No parsing context
        }
    }
}
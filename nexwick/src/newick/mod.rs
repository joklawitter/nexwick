//! Newick format parser and writer for phylogenetic trees.
//!
//! This module provides [NewickParser] to parse Newick format strings
//! into tree structures. The parser uses a
//! [TreeBuilder](crate::model::TreeBuilder) internally, which also resolves
//! labels. It may be used directly to parse Newick strings or when parsing a
//! Nexus file.
//!
//! # Quick API
//! For simple use cases with default settings:
//! * [`parse_file`] - parses a file, returns [CompactTree]s + [LeafLabelMap]
//! * [`parse_str`] - parses a single string, returns a [SimpleTree]
//!
//! # Full API
//! For more control, configure a [NewickParser] and
//! provide data via a [ByteParser]:
//! * [`NewickParser::parse_str`] - parse a single tree
//! * [`NewickParser::parse_all`] - parse all trees until EOF
//! * [`NewickParser::into_iter`] - obtain an iterator over trees
//!
//! # Format
//! The Newick format has the following simple grammar:
//! * `tree ::= vertex ';'`
//! * `vertex ::= leaf | internal_vertex`
//! * `internal_vertex ::= '(' vertex ',' vertex ')' [branch_length]`
//! * `leaf ::= label [branch_length]`
//! * `branch_length ::= ':' number`
//!
//! Furthermore:
//! * Whitespace can occur between elements,
//!   just not within an unquoted label or a branch_length
//! * Even newlines can occur anywhere except in labels (quoted and unquoted)
//! * Comments are square brackets and can occur anywhere where newlines are allowed
//!
//! In the extended Newick format, there can be comment-like annotation:
//! * `[@pop_size=0.543,color=blue]`
//!
//! For a leaf:
//! * label \[annotation\] \[branch_length\]
//!   - Example: A\[@pop_size=0.543\]:2.1
//!
//! For an internal vertex and the root:
//! * (children) \[annotation\] \[branch_length\]
//!   - Example: (A,B\[@pop_seize=0.345\]:6.7
//!
//! These are considered comments for now and skipped.

mod defs;
pub mod parser;
pub mod writer;

pub use parser::{NewickIterator, NewickParser};
pub use writer::{NewickStyle, to_newick, write_newick_file};

use crate::model::{CompactTree, LeafLabelMap, SimpleTree};
use crate::parser::ParsingError;
use crate::parser::byte_parser::ByteParser;
use std::path::Path;

// ============================================================================
// QUICK PARSING API (pub)
// ============================================================================
/// Parses a Newick file eagerly and returns all trees (as [CompactTree])
/// together with their shared [label mapping](LeafLabelMap).
///
/// This is a convenience function to parse a file containing
/// semicolon-separated list of Newick strings,
/// using default settings and thus not requiring configuration of a parser.
///
/// # Arguments
/// * `path` - Path to the file (accepting `&str`, `String`, `Path`, or `PathBuf`)
///   with semicolon-separated list of Newick strings
///
/// # Returns
/// * `(Vec<CompactTree>, LeafLabelMap)` - All parsed trees and their shared label mapping
/// * [ParsingError] - If file reading fails or Newick format is invalid
///
/// # Format
/// Expects standard Newick format with trees separated by semicolons.
/// Multiple trees can appear on the same line or across multiple lines,
/// and `[...]` comments and whitespace are fine.
///
/// # Example
/// ```no_run
/// use nexwick::newick::parse_file;
///
/// let (trees, label_map) = parse_file("anseriformes.nwk")?;
/// println!("Parsed {} trees with {} taxa", trees.len(), label_map.num_labels());
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_file<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<CompactTree>, LeafLabelMap), ParsingError> {
    // Set up byte parser
    let byte_parser = ByteParser::from_file_buffered(path)?;

    // Parse all trees
    let mut newick_parser = NewickParser::new_compact_defaults();
    let trees = newick_parser.parse_all(byte_parser)?;
    let label_map = newick_parser.into_label_storage();
    Ok((trees, label_map))
}

/// Parses a single Newick string to obtain a [SimpleTree].
///
/// This is a convenience function for quick parsing of a single Newick string
/// using default settings and thus not requiring configuration of a parser.
///
/// # Arguments
/// * `newick` - The Newick format string to parse
///
/// # Returns
/// * [SimpleTree] - Tree parsed from the string
/// * [ParsingError] - If the string is not valid Newick format
///
/// # Example
/// ```no_run
/// use nexwick::newick::parse_str;
///
/// let tree = parse_str("(Fratercula_cirrhata,(Fratercula_arctica,Fratercula_corniculata));")?;
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_str<S: AsRef<str>>(newick: S) -> Result<SimpleTree, ParsingError> {
    let mut newick_parser = NewickParser::new_simple_defaults();
    let mut byte_parser = ByteParser::for_str(newick.as_ref());
    newick_parser.parse_str(&mut byte_parser)
}

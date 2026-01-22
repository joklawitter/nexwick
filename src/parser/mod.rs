//! Parsers for phylogenetic tree file formats.
//!
//! This module provides parsers for reading phylogenetic tree files in
//! NEXUS and Newick formats, along with supporting infrastructure for
//! low-level byte parser and error handling.

pub mod byte_parser;
pub(crate) mod byte_source;
pub mod parsing_error;
pub mod utils;

// pub use utils::*;
// pub use byte_parser::*;
pub use parsing_error::ParsingError;
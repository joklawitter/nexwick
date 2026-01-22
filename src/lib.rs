//! Nexwick is library for parser phylogenetic trees from Nexus files and Newick strings.
//!
//! This crate provides configurable parser functionality for Nexus format phylogenetic tree files and Newick strings.
//! Core functionality provided:
//! - Nexus: Parse the taxa block and tree block of a nexus files (file ending ignored)
//! - Newick: Parse each Newick string in a file or single Newick strings.
//! - Tree builder: You can use the provided tree models or provide your own TreeBuilder trait implementation.
//! - Tree models:
//!   - Trees + Label Mapping: All trees share a index-to-label mapping and each leaf only stores a label index.
//!   - Leaf-labeled trees (LabeledTree): Each tree stores the label of each leaf directly in the leaf.
//!   - Both models use arena pattern and so no direct vertex references are stored, only vertex indices.
//! - Configurability:
//!   - Eager parser (all at once) or lazy (providing an iterator)
//!   - Burnin: number/percentage of initial trees skipped
//!   - Skip first: Since some Bayesian MCMC implementations include the start tree in Nexus files,
//! which is however not needed for analyzes, parser can be configured to directly skip first tree.
//!   - Loading full file in memory (default) or buffered (for huge files)
//!
//! Limitations:
//! - Only binary trees
//! - Only leaf-labels considered
//! - Additional vertex data not considered yet
//!
//! # Usage patterns
//! Can parse files in two main ways:
//! 1. Using a ParserBuilder, configure Parser, provide source and initialize, parse trees and obtain leaf map.
//! 2. Several methods provide quick access to parser using default configurations.
//!   - `newick::parse_str(&str)` parses a single newick string, returning
//!
//! ## Example Parser Configuration
//! todo
//! ## Example Default Parser
//! todo
//!
//! # Tree model details
//! todo
//!
//! # Example
//! ```ignore
//! use nexus_parser::parse_nexus_file;
//!
//! let (trees, labels) = parse_nexus_file("phylo.trees")?;
//! println!("Loaded {} trees with {} taxa", trees.len(), labels.num_labels());
//! ```


pub mod model;
pub mod newick;
pub mod nexus;
pub mod parser;

use crate::parser::parsing_error::ParsingError;
use crate::model::leaf_label_map::LeafLabelMap;
use crate::model::SimpleTree;
use crate::model::CompactTree;
use std::path::Path;

// ============================================================================
// Quick Nexus API
// ============================================================================
/// Parses a NEXUS file using default settings,
/// returning a vector of [`CompactTree`] together with a [`LeafLabelMap`].
///
/// See [`nexus::parse_file`] for full documentation.
pub fn parse_nexus_file<P: AsRef<Path>>(path: P) -> Result<(Vec<CompactTree>, LeafLabelMap), ParsingError> {
    nexus::parse_file(path)
}

// ============================================================================
// Quick Newick API
// ============================================================================
/// Parse a Newick string using default settings,
/// returning a [`SimpleTree`].
///
/// See [`newick::parse_str`] for full documentation of this convenience function.
pub fn parse_newick_str<S: AsRef<str>>(newick: S) -> Result<SimpleTree, ParsingError> {
    newick::parse_str(newick)
}

/// Parse a file containing a semicolon-separated list of Newick strings
/// using default settings, returning a vector of [`CompactTree`] together
/// with their shared [`LeafLabelMap`].
///
/// See [`newick::parse_file`] for full documentation of this convenience function.
pub fn parse_newick_file<P: AsRef<Path>>(path: P) -> Result<(Vec<CompactTree>, LeafLabelMap), ParsingError> {
    newick::parse_file(path)
}

//! Nexwick is library to parse phylogenetic trees from Nexus files and
//! Newick strings.
//!
//! This crate offers configurable parser (and writer) functionality for
//! Nexus format files and Newick strings to parse phylogenetic trees.
//! Core functionality provided:
//! - Nexus: Parse the taxa block and tree block of a nexus files (file ending ignored).
//! - Newick: Parse each Newick string in a file or single Newick strings.
//! - Tree builder: You can use the provided tree models or
//!   provide your own TreeBuilder trait implementation.
//! - Tree models:
//!   - [CompactTree] + [LeafLabelMap]: All trees share a index-to-label mapping and
//!     each leaf only stores a label index.
//!   - [SimpleTree]: Each tree stores the label of each
//!     leaf directly in the leaf.
//!   - Both models use arena pattern and so no direct vertex references are
//!     stored, only vertex indices.
//!   - See [crate::model] for more details.
//! - Configurability:
//!   - Eager parser (all at once) or lazy (providing an iterator)
//!   - Burnin: number/percentage of initial trees skipped
//!   - Skip first: Since some Bayesian MCMC implementations include the start
//!     tree in Nexus files,
//! which is however not needed for analyzes, parser can be configured to
//!     directly skip first tree.
//!   - Loading full file in memory (default) and
//!     in the future also buffered (for huge files)
//!
//! Limitations:
//! - Only binary trees
//! - Only leaf-labels considered
//! - Additional vertex data not considered yet
//!
//! # Usage patterns
//! Can parse files in two main ways:
//! 1. Several methods provide quick access to parsers with default settings.
//!    See [crate::newick] and [crate::nexus] documentation.
//! 2. Configure a parser using
//!    [NexusParserBuilder](crate::nexus::NexusParserBuilder) or
//!    [NewickParser](crate::newick::NewickParser) for full
//!    control over parsing mode, burnin, tree builder, etc.
//!
//! ## Example Default Configuration
//!
//! Parse a single Newick string:
//! ```no_run
//! use nexwick::parse_newick_str;
//!
//! let tree = parse_newick_str("((A:0.1,B:0.2):0.3,C:0.4);").unwrap();
//! assert_eq!(tree.num_leaves(), 3);
//! ```
//!
//! Parse a Nexus file:
//! ```no_run
//! use nexwick::parse_nexus_file;
//!
//! let (trees, labels) = parse_nexus_file("phylo.trees").unwrap();
//! println!("Loaded {} trees with {} taxa", trees.len(), labels.num_labels());
//! ```
//!
//! ## Example Parser Configuration
//!
//! For more control, use configure a parser yourself:
//! ```no_run
//! use nexwick::nexus::{NexusParserBuilder, Burnin};
//!
//! let mut parser = NexusParserBuilder::for_file("mcmc_samples.trees")?
//!     .with_skip_first()                    // Skip start tree
//!     .with_burnin(Burnin::Percentage(0.1)) // Discard first 10%
//!     .eager()                              // Parse all upfront (default)
//!     .build()?;
//!
//! let (trees, labels) = parser.into_results()?;
//! println!("Loaded {} post-burnin trees", trees.len());
//! # Ok::<(), nexwick::parser::ParsingError>(())
//! ```

pub mod model;
pub mod newick;
pub mod nexus;
pub mod parser;

use crate::model::CompactTree;
use crate::model::SimpleTree;
use crate::model::leaf_label_map::LeafLabelMap;
use crate::parser::parsing_error::ParsingError;
use std::path::Path;

// ============================================================================
// Quick Nexus API
// ============================================================================
/// Parses a NEXUS file using default settings,
/// returning a vector of [CompactTree] together with a [LeafLabelMap].
///
/// See [`nexus::parse_file`] for full documentation.
pub fn parse_nexus_file<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<CompactTree>, LeafLabelMap), ParsingError> {
    nexus::parse_file(path)
}

// ============================================================================
// Quick Newick API
// ============================================================================
/// Parse a Newick string using default settings,
/// returning a [SimpleTree].
///
/// See [`newick::parse_str`] for full documentation of this convenience function.
pub fn parse_newick_str<S: AsRef<str>>(newick: S) -> Result<SimpleTree, ParsingError> {
    newick::parse_str(newick)
}

/// Parse a file containing a semicolon-separated list of Newick strings
/// using default settings, returning a vector of [CompactTree] together
/// with their shared [LeafLabelMap].
///
/// See [`newick::parse_file`] for full documentation of this convenience function.
pub fn parse_newick_file<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<CompactTree>, LeafLabelMap), ParsingError> {
    newick::parse_file(path)
}

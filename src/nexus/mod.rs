//! NEXUS format parser and writer for phylogenetic trees.
//!
//! This module mainly provides a [`NexusParser`] and [NexusParserBuilder] for reading NEXUS files
//! containing phylogenetic trees. Supports both eager (load all trees) and lazy (parse on-demand)
//! parser modes, with options for handling burnin and skipping initial trees.
//!
//! This module provides:
//! - [NexusParserBuilder] / [NexusParser] — for reading NEXUS files
//! - [NexusWriter] — for writing NEXUS files
//!
//! Supports eager (load all trees) and lazy (parse on-demand) modes,
//! with options for burnin and skipping initial trees.
//!
//! # Quick API
//! For simple use cases with default settings:
//! - [`parse_file`] — parses a file, returns [`CompactTree`]s + [`LeafLabelMap`]
//!
//! # Format
//! A NEXUS file typically contains:
//! - A TAXA block defining the species/labels
//! - A TREES block containing multiple phylogenetic trees
//! - Optional TRANSLATE commands mapping short keys to full taxon labels
//!
//! ## Assumptions
//! * A `TAXA` and a `TREES` block are present, in this order
//! * A `TRANSLATE` command, if present, precedes any `TREE` command, with following details:
//!   - Command is a comma seperated list of pairs of "id/short label":
//!         `TRANSLATE [<key1=short1/id1> <label1>, ...];`
//!   - Mapping should *consistently* use integer or shorts as key; behaviour undefined otherwise
//!   - `<label>` must match a label provided in `TAXA` blog.
//!   - Length of mapping must match number of taxa/labels.
//!         (This is a program specific requirement, not of NEXUS files.)
//!   - A label with a space in it must be enclosed in single quotes and ...
//!   - A label with an apostrophe in it must be enclosed in single quotes
//!     and the apostrophe must be escaped with an apostrophe/single quote:
//!     e.g. `Wilson's Storm-petrel` becomes `'Wilson''s storm-petrel'`
//!   - No comments within pair allowed, only between comma and next pair,
//!     e.g. `[cool seabird] stormy 'Wilson''s_storm-petrel',`
//! * Trees come in semicolon separated list of tree commands
//! * One tree command has format `tree <name> = <Newick string>;`
//!   - Each pair is separated by a comma, optional whitespace and comments
//!   - Only one mapping per taxon allowed
//!   - Same label rules apply

mod defs;
mod parser;
mod writer;

pub use self::parser::{NexusParserBuilder, NexusParser, Burnin};
pub use self::writer::NexusWriter;

use crate::ParsingError;
use crate::LeafLabelMap;
use crate::CompactTree;
use std::path::Path;

// ============================================================================
// QUICK PARSING API (public)
// ============================================================================
/// Parses a Nexus file early and returns all trees (as [`CompactTree`])
/// and together with their [label mapping](LeafLabelMap`).
/// 
/// This is a convenience function to parse a file in Nexus format containing
/// at least a TAXA and a TREE block, with optional TRANSLATE command.
///
/// # Arguments
/// * `path` - Path to the file (accepting `&str`, `String`, `Path`, or `PathBuf`)
///
/// # Returns
/// A tuple of (trees, label_map) containing all parsed trees and their shared label mapping
///
/// # Errors
/// Returns an error if the file cannot be opened or parsed.
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<(Vec<CompactTree>, LeafLabelMap), ParsingError> {
    let nexus_parser = NexusParserBuilder::for_file(path)?
        .eager().build()?;
    let (trees, map) = nexus_parser.into_results()?;

    Ok((trees, map))
}

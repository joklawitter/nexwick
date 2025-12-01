//! NEXUS phylogenetic tree file parser.
//!
//! This crate provides parsing functionality for NEXUS format phylogenetic tree files,
//! commonly used in Bayesian phylogenetics (BEAST2, MrBayes, RevBayes).
//!
//! # Example
//! ```ignore
//! use nexus_parser::parse_nexus_file;
//!
//! let (trees, labels) = parse_nexus_file("phylo.trees")?;
//! println!("Loaded {} trees with {} taxa", trees.len(), labels.num_labels());
//! ```

/// Phylogenetic tree and data structures
pub mod model;
/// NEXUS and Newick format parsers
pub mod parser;

use crate::model::leaf_label_map::LeafLabelMap;
use crate::model::tree::Tree;
use crate::parser::nexus::NexusParserBuilder;
use std::error::Error;
use std::fs::File;

/// Parses a NEXUS file and returns all trees and their label mapping.
///
/// This is a convenience function that reads the entire file into memory
/// and parses all trees eagerly.
///
/// # Arguments
/// * `path` - Path to the NEXUS file
///
/// # Returns
/// A tuple of (trees, label_map) containing all parsed trees and their shared label mapping
///
/// # Errors
/// Returns an error if the file cannot be opened or parsed
pub fn parse_nexus_file(path: &str) -> Result<(Vec<Tree>, LeafLabelMap), Box<dyn Error>> {
    let nexus_parser = NexusParserBuilder::for_file(File::open(path).unwrap())?
        .eager().build()?;
    let (trees, map) = nexus_parser.into_results()?;

    Ok((trees, map))
}
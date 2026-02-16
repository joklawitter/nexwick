//! NEXUS format constants and definitions.
//!
//! This module contains byte string constants for parser and writing phylogenetic
//! tree files in NEXUS and Newick formats, as well as enum definitions for NEXUS blocks.

/// NEXUS label delimiters: comma, semicolon, whitespace
pub(crate) const NEXUS_LABEL_DELIMITERS: &[u8] = b" ,;\t\n\r";

/// NEXUS file header "#NEXUS"
pub(crate) const NEXUS_HEADER: &[u8] = b"#NEXUS";

/// NEXUS block begin keyword "Begin"
pub(crate) const BLOCK_BEGIN: &[u8] = b"Begin";

/// NEXUS block end keyword "End;" (with semicolon)
pub(crate) const BLOCK_END: &[u8] = b"End;";

// Taxa block keywords
/// TAXA block identifier "taxa;" (with semicolon)
pub(crate) const TAXA: &[u8] = b"taxa;";

/// TAXA block dimensions keyword "Dimensions"
pub(crate) const DIMENSIONS: &[u8] = b"Dimensions";

/// Number of taxa parameter "ntax"
pub(crate) const NTAX: &[u8] = b"ntax";

/// Tax labels command "Taxlabels"
pub(crate) const TAXLABELS: &[u8] = b"Taxlabels";

// Trees block keywords
/// TREES block identifier "trees;" (with semicolon)
pub(crate) const TREES: &[u8] = b"trees;";

/// TREES block translate command "Translate"
pub(crate) const TRANSLATE: &[u8] = b"Translate";

/// Individual tree declaration keyword "tree"
pub(crate) const TREE: &[u8] = b"tree";

/// NEXUS block types
#[derive(Debug, PartialEq, Clone)]
pub enum NexusBlock {
    Taxa,
    Trees,
    Data,
    Characters,
    Distances,
    Sets,
    Assumptions,
    UnknownBlock(String),
}

impl NexusBlock {
    /// Parse a block name (case-insensitive) into a NexusBlock variant
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "taxa" => NexusBlock::Taxa,
            "trees" => NexusBlock::Trees,
            "data" => NexusBlock::Data,
            "characters" => NexusBlock::Characters,
            "distances" => NexusBlock::Distances,
            "sets" => NexusBlock::Sets,
            "assumptions" => NexusBlock::Assumptions,
            _ => NexusBlock::UnknownBlock(name.to_string()),
        }
    }
}

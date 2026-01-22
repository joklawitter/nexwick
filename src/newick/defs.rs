//! Constants and definitions for Newick parser.
//!
//! This module contains byte string constants for parser and writing phylogenetic
//! tree files in NEXUS and Newick formats, as well as enum definitions for NEXUS blocks.

/// Newick label delimiters: parentheses, comma, colon, semicolon, whitespace
pub(crate) const NEWICK_LABEL_DELIMITERS: &[u8] = b"([,:; \n\t\r)]";

/// Default guess for number of leaves, when unknown
pub(crate) const DEFAULT_NUM_LEAVES_GUESS: usize = 10;
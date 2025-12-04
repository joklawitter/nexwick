/// Nexus label parsing delimiters: parentheses, comma, colon, semicolon, whitespace
pub(crate) const NEXUS_LABEL_DELIMITERS: &[u8] = b" ,;\t\n\r";

pub(crate) const NEXUS_HEADER: &[u8] = b"#NEXUS";

pub(crate) const BLOCK_BEGIN: &[u8] = b"Begin";

pub(crate) const BLOCK_END: &[u8] = b"End;";

// Taxa block
pub(crate) const TAXA: &[u8] = b"taxa;";

pub(crate) const DIMENSIONS: &[u8] = b"Dimensions";

pub(crate) const NTAX: &[u8] = b"ntax";

pub(crate) const TAXLABELS: &[u8] = b"Taxlabels";

// Tree block
pub(crate) const TREES: &[u8] = b"trees;";

pub(crate) const TRANSLATE: &[u8] = b"Translate";

pub(crate) const TREE: &[u8] = b"tree";

/// TODO
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
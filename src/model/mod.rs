//! Data model for binary phylogenetic trees.
//!
//! # Tree representation
//! Trees are represented by [GenTree], which uses the arena pattern to store
//! [Vertex] nodes. Each vertex is either a `Root`, `Internal`, or `Leaf`,
//! referenced by [VertexIndex]. Struct thus restricted to trees with
//! at least two leaves.
//!
//! Two concrete tree types are provided:
//!
//! | Type | Label storage | Use case |
//! |------|---------------|----------|
//! | [CompactTree] | [LabelIndex] into shared [LeafLabelMap] | Multiple trees with same taxa |
//! | [SimpleTree] | Owned [String] per leaf | Single self-contained tree |
//!
//! # Building trees
//! Trees are typically constructed during parsing via the [TreeBuilder]
//! trait, which decouples parsers from concrete tree types:
//!
//! - [CompactTreeBuilder] → [CompactTree]
//! - [SimpleTreeBuilder] → [SimpleTree]
//!
//! You can implement [TreeBuilder] to construct your own tree representation
//! while reusing the library's parsers.
//!
//! # Label handling
//! During parsing, labels flow through:
//! 1. [LabelResolver] — translates Newick strings
//!    (according to Nexus TRANSLATE command)
//! 2. [LabelStorage] — stores labels and returns references for tree leaves
//!
//! See the [tree_builder] module docs for details on this flow.

pub mod compact_tree_builder;
pub mod label_resolver;
pub mod label_storage;
pub mod leaf_label_map;
pub mod simple_tree_builder;
pub mod tree;
pub mod tree_builder;
pub mod vertex;

// Tree (generic)
pub use tree::GenTree;
pub use tree::VertexIndex;
pub use tree_builder::TreeBuilder;
pub use vertex::Vertex;
// Compact tree
pub use compact_tree_builder::CompactTreeBuilder;
pub use leaf_label_map::LabelIndex;
pub use leaf_label_map::LeafLabelMap;
pub use tree::CompactTree;
// Simple Tree
pub use simple_tree_builder::SimpleLabelStorage;
pub use simple_tree_builder::SimpleTreeBuilder;
pub use tree::SimpleTree;
// Label handling
pub use label_resolver::LabelResolver;
pub use label_storage::LabelStorage;

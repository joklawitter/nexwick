//! Model module for representing (binary) phylogenetic trees.
//!
//! TODO 1. provides tree model and two default implementations, 2. TreeBuilder can be used as own model
//!
//! A tree uses the arena pattern to store vertices,
//! which come in variety `root`, `internal`, and `leaf`.
//! A bidirectional map (LeafLabelMap) between leaf labels and indices is used
//! to share label information (as intended use is large numbers of trees).

pub mod tree;
pub mod vertex;
pub mod leaf_label_map;
pub mod tree_builder;
pub mod compact_tree_builder;
pub mod simple_tree_builder;
pub mod label_resolver;

pub use leaf_label_map::LabelIndex;
pub use leaf_label_map::LeafLabelMap;
pub use label_resolver::LabelResolver;
pub use tree::GenTree;
pub use tree::CompactTree;
pub use compact_tree_builder::CompactTreeBuilder;
pub use tree::SimpleTree;
pub use simple_tree_builder::SimpleTreeBuilder;
pub use tree::VertexIndex;
pub use vertex::Vertex;

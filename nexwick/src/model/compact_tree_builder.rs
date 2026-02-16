//! Provides [TreeBuilder] implementation structs for [CompactTree].

use crate::model::label_storage::LabelStorage;
use crate::model::tree_builder::TreeBuilder;
use crate::model::vertex::BranchLength;
use crate::model::{CompactTree, LabelIndex, LeafLabelMap, VertexIndex};

/// Builder that constructs [CompactTree] instances.
///
/// [CompactTreeBuilder] implements [TreeBuilder] to construct
/// [CompactTree] instances during parsing. Labels are stored externally in
/// a [LeafLabelMap], with leaves holding only [LabelIndex] references.
///
/// This is the recommended builder for parsing multiple trees from the same
/// file, as all trees share a single label map, avoiding string duplication.
///
/// # Example
/// ```no_run
/// use nexwick::newick::NewickParser;
/// use nexwick::parser::byte_parser::ByteParser;
///
/// let mut byte_parser = ByteParser::for_str("(A,(B,C));");
/// let mut parser = NewickParser::new_compact_defaults();
/// let trees = parser.parse_all(byte_parser)?;
/// let labels = parser.into_label_storage();
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct CompactTreeBuilder {
    current_tree: Option<CompactTree>,
}

impl CompactTreeBuilder {
    /// Creates a new builder in the empty state.
    pub fn new() -> Self {
        Self { current_tree: None }
    }
}

impl Default for CompactTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeBuilder for CompactTreeBuilder {
    type LabelRef = LabelIndex;
    type VertexIdx = VertexIndex;
    type Tree = CompactTree;
    type Storage = LeafLabelMap;

    fn create_storage(capacity: usize) -> LeafLabelMap {
        LeafLabelMap::with_capacity(capacity)
    }

    fn init_next(&mut self, num_leaves: usize) {
        self.current_tree = Some(CompactTree::new(num_leaves));
    }

    fn add_leaf(&mut self, branch_len: Option<f64>, label: LabelIndex) -> Self::VertexIdx {
        let tree = self.current_tree.as_mut().expect("init not called");
        tree.add_leaf(branch_len.map(BranchLength::new), label)
    }

    fn add_internal(
        &mut self,
        children: (Self::VertexIdx, Self::VertexIdx),
        branch_len: Option<f64>,
    ) -> Self::VertexIdx {
        let tree = self.current_tree.as_mut().expect("init not called");
        tree.add_internal_vertex(children, branch_len.map(BranchLength::new))
    }

    fn add_root(
        &mut self,
        children: (Self::VertexIdx, Self::VertexIdx),
        branch_len: Option<f64>,
    ) -> Self::VertexIdx {
        let tree = self.current_tree.as_mut().expect("init not called");
        tree.add_root(children, branch_len.map(BranchLength::new))
    }

    fn set_name(&mut self, tree_name: String) {
        if let Some(tree) = &mut self.current_tree {
            tree.set_name(tree_name);
        }
    }

    fn finish_tree(&mut self) -> Option<Self::Tree> {
        self.current_tree.take()
    }
}

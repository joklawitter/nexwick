//! TODO

use crate::model::label_resolver::LabelStorage;
use crate::parser::parsing_error::{ParsingError, ParsingErrorType};
use crate::model::tree_builder::TreeBuilder;
use crate::model::vertex::BranchLength;
use crate::model::{CompactTree, LabelIndex, LeafLabelMap, VertexIndex};
use crate::model::label_resolver::{LabelResolver, SimpleLabelStorage};

/// Default guess for number of leaves when unknown
const DEFAULT_NUM_LEAVES_GUESS: usize = 10;

// TODO
pub struct CompactTreeBuilder {
    current_tree: Option<CompactTree>,
}

impl CompactTreeBuilder {
    // TODO
    pub fn new() -> Self {
        Self {
            current_tree: None,
        }
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

    fn add_internal(&mut self, children: (Self::VertexIdx, Self::VertexIdx), branch_len: Option<f64>) -> Self::VertexIdx {
        let tree = self.current_tree.as_mut().expect("init not called");
        tree.add_internal_vertex(children, branch_len.map(BranchLength::new))
    }

    fn add_root(&mut self, children: (Self::VertexIdx, Self::VertexIdx), branch_len: Option<f64>) -> Self::VertexIdx {
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



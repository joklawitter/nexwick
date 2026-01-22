use crate::model::label_resolver::LabelStorage;
use crate::model::label_resolver::SimpleLabelStorage;
use crate::parser::parsing_error::{ParsingError, ParsingErrorType};
use crate::model::tree_builder::TreeBuilder;
use crate::model::vertex::BranchLength;
use crate::model::{LabelIndex, SimpleTree, VertexIndex};

pub struct SimpleTreeBuilder {
    current_tree: Option<SimpleTree>,
}

impl SimpleTreeBuilder {
    pub fn new() -> Self {
        Self { current_tree: None }
    }
}

impl TreeBuilder for SimpleTreeBuilder {
    type LabelRef = String;
    type VertexIdx = VertexIndex;
    type Tree = SimpleTree;
    type Storage = SimpleLabelStorage;

    fn create_storage(capacity: usize) -> SimpleLabelStorage {
        SimpleLabelStorage::with_capacity(capacity)
    }

    fn init_next(&mut self, num_leaves: usize) {
        self.current_tree = Some(SimpleTree::new(num_leaves));
    }

    fn add_leaf(&mut self, branch_len: Option<f64>, label: Self::LabelRef) -> Self::VertexIdx {
        // let tree = self.current_tree.as_mut().expect("init not called");
        // tree.add_leaf(branch_len.map(BranchLength::new), label)
        todo!();
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
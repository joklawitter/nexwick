//! Provides [TreeBuilder] implementation structs for [SimpleTree].

use crate::model::label_storage::LabelStorage;
use crate::model::tree_builder::TreeBuilder;
use crate::model::vertex::BranchLength;
use crate::model::{SimpleTree, VertexIndex};

/// Builder that constructs [SimpleTree] instances.
///
/// Each leaf stores its label as an owned [String], making trees fully
/// self-contained. This is simpler but less memory-efficient when parsing
/// multiple trees with the same taxa;
/// see [CompactTreeBuilder](crate::model::CompactTreeBuilder)
/// for an alternative.
///
/// Uses [SimpleLabelStorage] for label handling.
pub struct SimpleTreeBuilder {
    current_tree: Option<SimpleTree>,
}

impl SimpleTreeBuilder {
    /// Creates a new builder in the empty state.
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

// =#========================================================================#=
// SIMPLE LABEL STORAGE
// =#========================================================================S=
/// Basic [LabelStorage] implementation using owned strings.
///
/// Stores labels in a [`Vec<String>`] and returns cloned strings as
/// references. Simple but involves string allocation on each operation.
///
/// For more efficient storage with shared labels across trees,
/// see [LeafLabelMap](crate::model::LeafLabelMap).
#[derive(Debug, Default)]
pub struct SimpleLabelStorage {
    labels: Vec<String>,
}

impl LabelStorage for SimpleLabelStorage {
    type LabelRef = String;

    fn with_capacity(num_labels: usize) -> Self {
        Self {
            labels: Vec::with_capacity(num_labels),
        }
    }

    fn store_and_ref(&mut self, label: &str) -> String {
        self.labels.push(label.to_string());
        label.to_string()
    }

    fn check_and_ref(&self, label: &str) -> Option<String> {
        if self.labels.iter().any(|l| l == label) {
            Some(label.to_string())
        } else {
            None
        }
    }

    fn index_to_ref(&self, index: usize) -> String {
        self.labels[index].clone()
    }

    fn num_labels(&self) -> usize {
        self.labels.len()
    }
}

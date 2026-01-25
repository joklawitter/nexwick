#![allow(unused)]

use nexwick::model::vertex::BranchLength;
use nexwick::model::{LabelIndex, Vertex};

// ============= Branch Length Tests =============
#[test]
fn test_branch_lengths() {
    let test_length = 1.234;
    let vertex: Vertex<LabelIndex> =
        Vertex::new_internal(5, (1, 2), Some(BranchLength::new(test_length)));
    assert_eq!(*vertex.branch_length().unwrap(), test_length);
}

#[test]
#[should_panic]
fn test_negative_branch_length() {
    let negative_length = BranchLength::new(-1.0);
}

// ============= Vertex Variant Consistency Tests =============
#[test]
fn test_is_x() {
    let leaf = Vertex::new_leaf(0, Some(BranchLength::new(0.5)), 10);
    assert!(leaf.is_leaf());

    let vertex: Vertex<LabelIndex> = Vertex::new_internal(0, (1, 2), Some(BranchLength::new(0.5)));
    assert!(vertex.is_internal());

    let root: Vertex<LabelIndex> = Vertex::new_root_without_branch(2, (42, 42));
    assert!(root.is_root());
}

#[test]
fn test_nonleaf_vertex_has_no_label() {
    let internal: Vertex<LabelIndex> =
        Vertex::new_internal(0, (1, 2), Some(BranchLength::new(0.5)));
    assert_eq!(internal.label(), None);

    let root: Vertex<LabelIndex> = Vertex::new_root(0, (12, 34), Some(BranchLength::new(0.6)));
    assert_eq!(root.label(), None);
}

#[test]
fn test_parent_unset() {
    let vertex: Vertex<LabelIndex> = Vertex::new_internal(0, (1, 2), Some(BranchLength::new(0.5)));
    assert_eq!(vertex.parent_index(), None);
    assert!(!vertex.has_parent());

    let leaf: Vertex<LabelIndex> = Vertex::new_leaf(0, Some(BranchLength::new(0.5)), 0);
    assert_eq!(leaf.parent_index(), None);
    assert!(!leaf.has_parent());

    let root: Vertex<LabelIndex> = Vertex::new_root_without_branch(2, (42, 42));
    assert_eq!(root.parent_index(), None);
}

#[test]
fn test_leaf_has_no_children() {
    let vertex: Vertex<LabelIndex> = Vertex::new_leaf(0, Some(BranchLength::new(0.5)), 42);
    assert_eq!(vertex.children(), None);
}

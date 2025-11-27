use nexus_parser::model::tree::{LeafLabelMap, Tree};
use nexus_parser::model::vertex::BranchLength;

#[test]
fn test_building_tree() {
    let mut tree = Tree::new(3);
    let index_l1 = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let index_l2 = tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    let index_l3 = tree.add_leaf(Some(BranchLength::new(0.5)), 2);
    let index_i1 = tree.add_internal_vertex((index_l1, index_l2), Some(BranchLength::new(1.5)));
    let index_root = tree.add_root((index_l3, index_i1));

    // Counts
    assert_eq!(tree.num_leaves(), 3);
    assert_eq!(tree.num_internal(), 1);
    assert_eq!(tree.num_vertices(), 5);

    // Root
    assert_eq!(tree.root().index(), index_root);
    let root = tree.root();
    assert_eq!(root.index(), index_root);
    assert!(root.is_root());

    // Leaf
    let l2 = &tree[index_l2];
    assert!(l2.is_leaf());
    assert_eq!(l2.index(), index_l2);
    assert_eq!(l2.label_index().unwrap(), 1);

    // Internal
    let inti = &tree[index_i1];
    assert!(inti.is_internal());
    assert_eq!(inti.index(), index_i1);
    assert_eq!(inti.branch_length().unwrap(), BranchLength::new(1.5));
}

#[test]
#[should_panic]
fn test_get_root_panics_on_empty_tree() {
    let tree = Tree::new(2);
    tree.root(); // Should panic
}

#[test]
#[should_panic]
fn test_get_vertex_out_of_bounds() {
    let tree = Tree::new(2);
    let _ = &tree[55];
}


// ============= LeafLabelMap Tests =============

#[test]
fn test_get_or_insert_new_label() {
    let mut map = LeafLabelMap::new(5);
    let index_wrybill = map.get_or_insert("Anarhynchus frontalis");
    assert_eq!(index_wrybill, 0);
    assert!(map.contains_label("Anarhynchus frontalis"));
}

#[test]
fn test_get_or_insert_increments_index() {
    let mut map = LeafLabelMap::new(5);
    let index_kaki = map.get_or_insert("Himantopus novaezelandiae");
    let index_pied = map.get_or_insert("Himantopus leucocephalus");
    assert_eq!(index_kaki, 0);
    assert_eq!(index_pied, 1);
    assert_eq!(map.num_labels(), 2);
}

#[test]
fn test_get_or_insert_returns_same_index_for_duplicate() {
    let mut map = LeafLabelMap::new(5);
    let index_kakapo = map.get_or_insert("Strigops habroptilus");
    let index_kea = map.get_or_insert("Nestor notabilis");
    let index_kaka = map.get_or_insert("Nestor meridionalis");
    let index_popoka = map.get_or_insert("Strigops habroptilus");

    assert_eq!(index_kakapo, index_popoka);
    assert_ne!(index_kakapo, index_kea);
    assert_ne!(index_kakapo, index_kaka);
    assert_eq!(map.num_labels(), 3);
}

#[test]
fn test_get_label_returns_correct_label() {
    let mut map = LeafLabelMap::new(5);
    let index_rock_wren = map.get_or_insert("Xenicus gilviventris");
    assert_eq!(map.get_label(index_rock_wren), Some("Xenicus gilviventris"));
}

#[test]
fn test_get_label_returns_none_for_invalid_index() {
    let map = LeafLabelMap::new(5);
    assert_eq!(map.get_label(0), None);
}


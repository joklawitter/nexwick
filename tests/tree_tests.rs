use nexwick::model::LabelIndex;
use nexwick::model::leaf_label_map::LeafLabelMap;
use nexwick::model::tree::GenTree;
use nexwick::model::vertex::BranchLength;
use nexwick::newick::NewickStyle;

// ============= Tree Construction Tests =============
#[test]
fn test_building_tree() {
    let mut tree: GenTree<LabelIndex> = GenTree::new(3);
    let index_l1 = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let index_l2 = tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    let index_l3 = tree.add_leaf(Some(BranchLength::new(0.5)), 2);
    let index_i1 = tree.add_internal_vertex((index_l1, index_l2), Some(BranchLength::new(1.5)));
    let index_root = tree.add_root_without_branch((index_l3, index_i1));

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
    assert_eq!(*l2.label().unwrap(), 1);

    // Internal
    let inti = &tree[index_i1];
    assert!(inti.is_internal());
    assert_eq!(inti.index(), index_i1);
    assert_eq!(inti.branch_length().unwrap(), BranchLength::new(1.5));
}

// ============= Tree Methods Tests =============
/// Builds an ultrametric test tree: ((A:1.0, B:1.0):1.0, C:2.0)
/// Height = 2.0, total branch length = 5.0
fn make_test_tree() -> (GenTree<LabelIndex>, LeafLabelMap) {
    let mut tree: GenTree<LabelIndex> = GenTree::new(3);
    let mut labels = LeafLabelMap::new(3);

    let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    let c = tree.add_leaf(Some(BranchLength::new(2.0)), labels.get_or_insert("C"));
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(1.0)));
    tree.add_root_without_branch((internal, c));

    (tree, labels)
}

#[test]
fn test_is_ultrametric() {
    let (tree, _) = make_test_tree();
    assert!(tree.is_ultrametric());
}

#[test]
fn test_is_ultrametric_false() {
    // Non-ultrametric: ((A:1.0, B:2.0):1.0, C:2.0)
    let mut tree: GenTree<LabelIndex> = GenTree::new(3);
    tree.add_leaf(Some(BranchLength::new(1.0)), 0); // A - path = 2.0
    tree.add_leaf(Some(BranchLength::new(2.0)), 1); // B - path = 3.0 (different!)
    tree.add_leaf(Some(BranchLength::new(2.0)), 2);
    tree.add_internal_vertex((0, 1), Some(BranchLength::new(1.0)));
    tree.add_root_without_branch((3, 2));

    assert!(!tree.is_ultrametric());
}

#[test]
fn test_height() {
    let (tree, _) = make_test_tree();
    assert!((tree.height() - 2.0).abs() < 1e-10);
}

#[test]
fn test_height_of() {
    let (tree, _) = make_test_tree();
    let internal = &tree[3]; // internal node
    assert!((tree.height_of(internal) - 1.0).abs() < 1e-10);
}

#[test]
fn test_name_and_set_name() {
    let (mut tree, _) = make_test_tree();
    assert!(tree.name().is_none());

    tree.set_name("Yew".to_string());
    assert_eq!(tree.name(), Some(&"Yew".to_string()));
}

#[test]
fn test_vertex_access() {
    let (mut tree, _) = make_test_tree();

    // vertex() and index operator return same reference
    let leaf = tree.vertex(0);
    assert!(std::ptr::eq(leaf, &tree[0]));

    // vertex_mut() allows modification - we can't change much, but we can access it
    let leaf_mut = tree.vertex_mut(0);
    leaf_mut.set_parent(leaf_mut.parent_index().unwrap());
    assert!(leaf_mut.is_leaf());
}

#[test]
fn test_total_branch_length() {
    let (tree, _) = make_test_tree();
    assert!((tree.total_branch_length() - 5.0).abs() < 1e-10);
}

#[test]
fn test_vertices_have_branch_lengths() {
    let (tree, _) = make_test_tree();
    assert!(tree.vertices_have_branch_lengths());
}

#[test]
fn test_vertices_have_branch_lengths_false() {
    let mut tree: GenTree<LabelIndex> = GenTree::new(2);
    tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    tree.add_leaf(None, 1); // No branch length
    tree.add_root_without_branch((0, 1));

    assert!(!tree.vertices_have_branch_lengths());
}

#[test]
fn test_is_valid() {
    let (tree, _) = make_test_tree();
    assert!(tree.is_valid());
}

#[test]
fn test_num_leaves_mismatch() {
    // Initialize tree for 5 leaves but only add 2
    let mut tree: GenTree<LabelIndex> = GenTree::new(5);
    tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    tree.add_root_without_branch((0, 1));

    assert_eq!(tree.num_leaves_init(), 5);
    assert_eq!(tree.num_leaves(), 2);
    assert!(tree.is_valid()); // Should still be valid
}

// ============= Tree Consistency Tests =============
#[test]
#[should_panic]
fn test_get_root_panics_on_empty_tree() {
    let tree: GenTree<LabelIndex> = GenTree::new(2);
    tree.root(); // Should panic
}

#[test]
#[should_panic]
fn test_get_vertex_out_of_bounds() {
    let tree: GenTree<LabelIndex> = GenTree::new(2);
    let _ = &tree[55];
}

// ============= Iterator Tests =============
#[test]
fn test_post_order_iter_visits_children_before_parents() {
    // Build tree: ((A:1.0,B:1.0):0.5,C:1.5):0.0;
    let mut tree: GenTree<LabelIndex> = GenTree::new(3);
    let a = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    let c = tree.add_leaf(Some(BranchLength::new(1.5)), 2);
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(0.5)));
    let root = tree.add_root_without_branch((internal, c));

    let visited: Vec<_> = tree.post_order_iter().map(|v| v.index()).collect();

    // In post-order: leaves first, then internal, then root
    assert_eq!(visited.len(), 5);

    // Leaves should come before internal node
    let a_pos = visited.iter().position(|&idx| idx == a).unwrap();
    let b_pos = visited.iter().position(|&idx| idx == b).unwrap();
    let c_pos = visited.iter().position(|&idx| idx == c).unwrap();
    let internal_pos = visited.iter().position(|&idx| idx == internal).unwrap();
    let root_pos = visited.iter().position(|&idx| idx == root).unwrap();

    assert!(a_pos < internal_pos);
    assert!(b_pos < internal_pos);
    assert!(internal_pos < root_pos);
    assert!(c_pos < root_pos);

    // Root should be last
    assert_eq!(visited[4], root);
}

#[test]
fn test_pre_order_iter_visits_parents_before_children() {
    // Build tree: ((A:1.0,B:1.0):0.5,C:1.5):0.0;
    let mut tree: GenTree<LabelIndex> = GenTree::new(3);
    let a = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    let c = tree.add_leaf(Some(BranchLength::new(1.5)), 2);
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(0.5)));
    let root = tree.add_root_without_branch((internal, c));

    let visited: Vec<_> = tree.pre_order_iter().map(|v| v.index()).collect();

    // In pre-order: root first, then children
    assert_eq!(visited.len(), 5);

    let a_pos = visited.iter().position(|&idx| idx == a).unwrap();
    let b_pos = visited.iter().position(|&idx| idx == b).unwrap();
    let c_pos = visited.iter().position(|&idx| idx == c).unwrap();
    let internal_pos = visited.iter().position(|&idx| idx == internal).unwrap();
    let root_pos = visited.iter().position(|&idx| idx == root).unwrap();

    // Root should be first
    assert_eq!(visited[0], root);

    // Parent before children
    assert!(root_pos < internal_pos);
    assert!(root_pos < c_pos);
    assert!(internal_pos < a_pos);
    assert!(internal_pos < b_pos);
}

#[test]
fn test_iter_on_empty_tree() {
    let tree: GenTree<LabelIndex> = GenTree::new(2);

    let post_count = tree.post_order_iter().count();
    let pre_count = tree.pre_order_iter().count();

    assert_eq!(post_count, 0);
    assert_eq!(pre_count, 0);
}

// ============= To Newick Tests =============
#[test]
fn test_to_newick_with_labels() {
    let mut tree: GenTree<LabelIndex> = GenTree::new(3);
    let mut labels = LeafLabelMap::new(3);

    let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    let c = tree.add_leaf(Some(BranchLength::new(2.0)), labels.get_or_insert("C"));
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(0.5)));
    tree.add_root_without_branch((internal, c));

    let newick = tree.to_newick(&NewickStyle::Label, Some(&labels));
    assert_eq!(newick, "((A:1,B:1):0.5,C:2);");
}

#[test]
fn test_to_newick_zero_indexed() {
    let mut tree: GenTree<LabelIndex> = GenTree::new(2);
    let a = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(2.0)), 1);
    tree.add_root_without_branch((a, b));

    let newick = tree.to_newick(&NewickStyle::ZeroIndexed, None);
    assert_eq!(newick, "(0:1,1:2);");
}

#[test]
fn test_to_newick_one_indexed() {
    let mut tree: GenTree<LabelIndex> = GenTree::new(2);
    let a = tree.add_leaf(Some(BranchLength::new(1.5)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(2.5)), 1);
    tree.add_root_without_branch((a, b));

    let newick = tree.to_newick(&NewickStyle::OneIndexed, None);
    assert_eq!(newick, "(1:1.5,2:2.5);");
}

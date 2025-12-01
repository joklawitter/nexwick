use nexus_parser::model::leaf_label_map::LeafLabelMap;
use nexus_parser::model::tree::{NewickStyle, Tree};
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

// ============= Iterator Tests =============

#[test]
fn test_post_order_iter_visits_children_before_parents() {
    // Build tree: ((A:1.0,B:1.0):0.5,C:1.5):0.0;
    let mut tree = Tree::new(3);
    let a = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    let c = tree.add_leaf(Some(BranchLength::new(1.5)), 2);
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(0.5)));
    let root = tree.add_root((internal, c));

    let visited: Vec<_> = tree.post_order_iter()
        .map(|v| v.index())
        .collect();

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
    let mut tree = Tree::new(3);
    let a = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), 1);
    let c = tree.add_leaf(Some(BranchLength::new(1.5)), 2);
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(0.5)));
    let root = tree.add_root((internal, c));

    let visited: Vec<_> = tree.pre_order_iter()
        .map(|v| v.index())
        .collect();

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
    let tree = Tree::new(2);

    let post_count = tree.post_order_iter().count();
    let pre_count = tree.pre_order_iter().count();

    assert_eq!(post_count, 0);
    assert_eq!(pre_count, 0);
}


// ============= To Newick Tests =============

#[test]
fn test_to_newick_with_labels() {
    let mut tree = Tree::new(3);
    let mut labels = LeafLabelMap::new(3);

    let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    let c = tree.add_leaf(Some(BranchLength::new(2.0)), labels.get_or_insert("C"));
    let internal = tree.add_internal_vertex((a, b), Some(BranchLength::new(0.5)));
    tree.add_root((internal, c));

    let newick = tree.to_newick(NewickStyle::Label, Some(&labels));
    assert_eq!(newick, "((A:1,B:1):0.5,C:2);");
}

#[test]
fn test_to_newick_zero_indexed() {
    let mut tree = Tree::new(2);
    let a = tree.add_leaf(Some(BranchLength::new(1.0)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(2.0)), 1);
    tree.add_root((a, b));

    let newick = tree.to_newick(NewickStyle::ZeroIndexed, None);
    assert_eq!(newick, "(0:1,1:2);");
}

#[test]
fn test_to_newick_one_indexed() {
    let mut tree = Tree::new(2);
    let a = tree.add_leaf(Some(BranchLength::new(1.5)), 0);
    let b = tree.add_leaf(Some(BranchLength::new(2.5)), 1);
    tree.add_root((a, b));

    let newick = tree.to_newick(NewickStyle::OneIndexed, None);
    assert_eq!(newick, "(1:1.5,2:2.5);");
}
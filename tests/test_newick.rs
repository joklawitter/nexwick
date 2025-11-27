use nexus_parser::parser::byte_parser::ByteParser;
use nexus_parser::parser::newick::NewickParser;

#[test]
fn test_basic_tree() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::from_str(newick);
    let mut newick_parser = NewickParser::new().with_num_leaves(3);
    let tree = newick_parser.parse(&mut parser).unwrap();
    let leaf_map = newick_parser.into_leaf_label_map();

    // Test counts
    assert_eq!(tree.num_leaves(), 3);
    assert_eq!(tree.num_internal(), 1);
    assert_eq!(tree.num_vertices(), 5);
    assert_eq!(leaf_map.num_labels(), 3);

    // Test basic label parsing
    assert!(leaf_map.contains_label("A"));
    assert!(leaf_map.contains_label("B"));
    assert!(leaf_map.contains_label("C"));

    // Test relationships
    // - Root has children (internal, C)
    let root = tree.root();
    let root_index = root.index();
    let (root_left, root_right) = root.children().unwrap();

    // - Internal node has children (A, B)
    let internal = tree.vertex(root_left);
    assert!(internal.is_internal());
    let (internal_left, internal_right) = internal.children().unwrap();

    // - Three leaves
    let leaf_a = tree.vertex(internal_left);
    let leaf_b = tree.vertex(internal_right);
    let leaf_c = tree.vertex(root_right);
    assert!(leaf_a.is_leaf());
    assert!(leaf_b.is_leaf());
    assert!(leaf_c.is_leaf());

    // - Parent relationships
    assert_eq!(internal.parent_index(), Some(root_index));
    assert_eq!(leaf_a.parent_index(), Some(root_left));
    assert_eq!(leaf_b.parent_index(), Some(root_left));
    assert_eq!(leaf_c.parent_index(), Some(root_index));
}

#[test]
fn test_basic_tree_without_root_branch() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0);";
    let mut parser = ByteParser::from_str(newick);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser).unwrap();

    // Test counts
    assert_eq!(tree.num_leaves(), 3);
    assert_eq!(tree.num_internal(), 1);
    assert_eq!(tree.num_vertices(), 5);
}

#[test]
fn test_tree_with_quoted_labels() {
    let newick = "(('Taxon one':1.5,'Second''s taxon':2.5):3.0,'3rd Taxon':4.0):0.0;";
    let mut parser = ByteParser::from_str(newick);
    let mut newick_parser = NewickParser::new().with_num_leaves(3);
    let tree = newick_parser.parse(&mut parser).unwrap();
    let leaf_map = newick_parser.into_leaf_label_map();

    assert_eq!(tree.num_leaves(), 3);
    assert!(leaf_map.contains_label("Taxon one"));
    assert!(leaf_map.contains_label("Second's taxon"));
    assert!(leaf_map.contains_label("3rd Taxon"));
}

#[test]
fn test_tree_with_scientific_notation() {
    let newick = "((A:1e-5,B:2.5E+3):1.0e2,C:3.14E-10):0.0;";
    let mut parser = ByteParser::from_str(newick);
    let mut newick_parser = NewickParser::new().with_num_leaves(3);
    let tree = newick_parser.parse(&mut parser).unwrap();
    let leaf_map = newick_parser.into_leaf_label_map();

    assert_eq!(tree.num_leaves(), 3);
    assert_eq!(tree.num_internal(), 1);
    assert_eq!(tree.num_vertices(), 5);
    assert_eq!(leaf_map.num_labels(), 3);
}

#[test]
fn test_optional_branch_length() {
    let newick = "((A:1.0,B),C:4.0);";
    let mut parser = ByteParser::from_str(newick);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser);
    assert!(tree.is_ok());
}

#[test]
fn test_newick_with_comment() {
    let newick_with_comment = "((A[Great Commentoran]:0.33,B[Pied Commentoran]:0.33):1.87,C:[King Commentoran]2.2):0.0";
    let mut parser = ByteParser::from_str(newick_with_comment);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser);
    assert!(tree.is_ok()); // TODO expected to fail as not implemented yet
}

// --- TESTS DEALING WITH CORRUPT NEWICK STRINGS ---

#[test]
fn test_missing_semicolon() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0):0.5";
    let mut parser = ByteParser::from_str(newick);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser);
    assert!(tree.is_err());
}

#[test]
fn test_missing_comma() {
    let newick = "((A:1.0 B:2.0):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::from_str(newick);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser);
    assert!(tree.is_err());
}

#[test]
fn test_unmatched_parentheses() {
    let newick = "((A:1.0,B:2.0:3.0,C:4.0):0.5;";
    let mut parser = ByteParser::from_str(newick);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser);
    assert!(tree.is_err());
}

#[test]
fn test_invalid_branch_length() {
    let newick = "((A:1.0,B:abc):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::from_str(newick);
    let tree = NewickParser::new().with_num_leaves(3).parse(&mut parser);
    assert!(tree.is_err());
}

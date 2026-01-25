use nexwick::newick::{NewickParser, parse_file};
use nexwick::parser::byte_parser::ByteParser;
use std::path::Path;

// --- TESTS NEWICK STRING PARSING ---
#[test]
fn test_basic_compact_tree() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::for_str(newick);
    let mut newick_parser = NewickParser::new_compact_defaults().with_num_leaves(3);
    let tree = newick_parser.parse_str(&mut parser).unwrap();
    let leaf_map = newick_parser.into_label_storage();

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
fn test_basic_simple_tree() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::for_str(newick);
    let mut newick_parser = NewickParser::new_simple_defaults().with_num_leaves(3);
    let tree = newick_parser.parse_str(&mut parser).unwrap();

    assert_eq!(tree.num_leaves(), 3);

    // Labels are stored directly in leaves
    let root = tree.root();
    let (left, right) = root.children().unwrap();
    let internal = tree.vertex(left);
    let (a_idx, b_idx) = internal.children().unwrap();

    assert_eq!(tree.vertex(a_idx).label().unwrap(), "A");
    assert_eq!(tree.vertex(b_idx).label().unwrap(), "B");
    assert_eq!(tree.vertex(right).label().unwrap(), "C");
}

#[test]
fn test_basic_tree_without_root_branch() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0);";
    let mut parser = ByteParser::for_str(newick);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser)
        .unwrap();

    // Test counts
    assert_eq!(tree.num_leaves(), 3);
    assert_eq!(tree.num_internal(), 1);
    assert_eq!(tree.num_vertices(), 5);
}

#[test]
fn test_tree_with_quoted_labels() {
    let newick = "(('Taxon one':1.5,'Second''s taxon':2.5):3.0,'3rd Taxon':4.0):0.0;";
    let mut parser = ByteParser::for_str(newick);
    let mut newick_parser = NewickParser::new_compact_defaults().with_num_leaves(3);
    let tree = newick_parser.parse_str(&mut parser).unwrap();
    let leaf_map = newick_parser.into_label_storage();

    assert_eq!(tree.num_leaves(), 3);
    assert!(leaf_map.contains_label("Taxon one"));
    assert!(leaf_map.contains_label("Second's taxon"));
    assert!(leaf_map.contains_label("3rd Taxon"));
}

#[test]
fn test_tree_with_scientific_notation() {
    let newick = "((A:1e-5,B:2.5E+3):1.0e2,C:3.14E-10):0.0;";
    let mut parser = ByteParser::for_str(newick);
    let mut newick_parser = NewickParser::new_compact_defaults().with_num_leaves(3);
    let tree = newick_parser.parse_str(&mut parser).unwrap();
    let leaf_map = newick_parser.into_label_storage();

    assert_eq!(tree.num_leaves(), 3);
    assert_eq!(tree.num_internal(), 1);
    assert_eq!(tree.num_vertices(), 5);
    assert_eq!(leaf_map.num_labels(), 3);
}

#[test]
fn test_optional_branch_length() {
    let newick = "((A:1.0,B),C:4.0);";
    let mut parser = ByteParser::for_str(newick);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);
    assert!(tree.is_ok());
}

#[test]
fn test_newick_with_comment_1() {
    let newick_with_comment = "[A tree of] (([Shags!]A[Great Commentoran]:0.33,B[Pied Commentoran]:0.33):1.87,C:[King Commentoran]2.2):0.0;";
    let mut parser = ByteParser::for_str(newick_with_comment);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);

    if tree.is_err() {
        eprintln!(
            "Error parsing tree with comments: {:?}",
            tree.as_ref().err()
        );
    }

    assert!(tree.is_ok());
}

#[test]
fn test_newick_with_comment_2() {
    let newick_with_comment = "[A tree of] ([Shags!] C:[King Commentoran] 2.2, (A[Great Commentoran]:0.33, B[Pied Commentoran]:0.33):1.87):0.0[The end.];";
    let mut parser = ByteParser::for_str(newick_with_comment);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);

    if tree.is_err() {
        eprintln!(
            "Error parsing tree with comments: {:?}",
            tree.as_ref().err()
        );
    }

    assert!(tree.is_ok());
}

// --- TESTS DEALING WITH CORRUPT NEWICK STRINGS ---

#[test]
fn test_missing_semicolon() {
    let newick = "((A:1.0,B:2.0):3.0,C:4.0):0.5";
    let mut parser = ByteParser::for_str(newick);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);
    assert!(tree.is_err());
}

#[test]
fn test_missing_comma() {
    let newick = "((A:1.0 B:2.0):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::for_str(newick);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);
    assert!(tree.is_err());
}

#[test]
fn test_unmatched_parentheses() {
    let newick = "((A:1.0,B:2.0:3.0,C:4.0):0.5;";
    let mut parser = ByteParser::for_str(newick);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);
    assert!(tree.is_err());
}

#[test]
fn test_invalid_branch_length() {
    let newick = "((A:1.0,B:abc):3.0,C:4.0):0.5;";
    let mut parser = ByteParser::for_str(newick);
    let tree = NewickParser::new_compact_defaults()
        .with_num_leaves(3)
        .parse_str(&mut parser);
    assert!(tree.is_err());
}

// --- TESTS PARSING WHOLE FILE ---
#[test]
fn test_parsing_newick_file() {
    let path = Path::new("tests")
        .join("fixtures")
        .join("newick_t4_n10.nwk");
    let (trees, leaf_map) = parse_file(path).unwrap();

    assert_eq!(trees.len(), 3);
    assert_eq!(leaf_map.num_labels(), 10);

    for tree in &trees {
        assert_eq!(tree.num_leaves(), 10);
        assert!(tree.is_valid());
    }
}

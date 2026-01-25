use nexwick::parse_nexus_file;
use nexwick::newick::NewickStyle;
use nexwick::nexus::{Burnin, NexusParserBuilder};
use std::path::Path;

#[test]
fn test_single_tree() {
    let path = Path::new("tests").join("fixtures").join("nexus_t1_n10.trees");
    let result = parse_nexus_file(path);
    if let Err(e) = &result {
        eprintln!("Error parsing single tree: {:?}", e);
    }
    assert!(result.is_ok());

    let (trees, leaf_map) = result.unwrap();
    assert_eq!(trees.len(), 1);
    assert_eq!(leaf_map.num_labels(), 10);

    let tree = &trees[0];
    assert_eq!(tree.num_leaves(), 10);
    assert!(tree.is_valid());
}

#[test]
fn test_multiple_trees_with_translate() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");
    let result = parse_nexus_file(path);
    if let Err(e) = &result {
        eprintln!("Error parsing multiple trees: {:?}", e);
    }
    assert!(result.is_ok());

    let (trees, leaf_map) = result.unwrap();
    assert_eq!(trees.len(), 11);
    assert_eq!(leaf_map.num_labels(), 20);

    for tree in &trees {
        assert_eq!(tree.num_leaves(), 20);
        assert!(tree.is_valid());
    }
}

#[test]
fn test_comments_and_unknown_blocks() {
    let path = Path::new("tests").join("fixtures").join("nexus_t3_n10_comments.trees");
    let result = parse_nexus_file(path);
    if let Err(e) = &result {
        eprintln!("Error parsing trees with lots of comments: {:?}", e);
    }
    assert!(result.is_ok());

    let (trees, leaf_map) = result.unwrap();
    assert_eq!(trees.len(), 3);
    assert_eq!(leaf_map.num_labels(), 10);

    for tree in &trees {
        assert_eq!(tree.num_leaves(), 10);
        assert!(tree.is_valid());
    }
}

#[test]
fn test_skip_first() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    let mut parser = NexusParserBuilder::for_file(path).unwrap()
        .with_skip_first()
        .eager()
        .build()
        .unwrap();

    // Should have 10 trees instead of 11 (skipped first)
    assert_eq!(parser.num_trees(), 10);
    assert_eq!(parser.num_total_trees(), 11);

    let mut count = 0;
    while let Some(tree) = parser.next_tree_ref() {
        assert_eq!(tree.num_leaves(), 20);
        assert!(tree.is_valid());
        count += 1;
    }
    assert_eq!(count, 10);

    // Also check if reset works for eager mode
    parser.reset();
    let mut count = 0;
    while let Some(tree) = parser.next_tree_ref() {
        assert_eq!(tree.num_leaves(), 20);
        assert!(tree.is_valid());
        count += 1;
    }
    assert_eq!(count, 10);
}

#[test]
fn test_burnin_count() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    let parser = NexusParserBuilder::for_file(path).unwrap()
        .with_burnin(Burnin::Count(5))
        .eager()
        .build()
        .unwrap();

    // Should have 6 trees after skipping 5 as burnin
    assert_eq!(parser.num_trees(), 6);
    assert_eq!(parser.num_total_trees(), 11);

    let (trees, _) = parser.into_results().unwrap();
    let mut count = 0;
    for tree in &trees {
        assert_eq!(tree.num_leaves(), 20);
        assert!(tree.is_valid());
        count += 1;
    }
    assert_eq!(count, 6);
}

#[test]
fn test_burnin_percentage() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    // 25% of 11 trees = 2.75 -> floor to 2
    let parser = NexusParserBuilder::for_file(path).unwrap()
        .with_burnin(Burnin::Percentage(0.25))
        .eager()
        .build()
        .unwrap();

    assert_eq!(parser.num_trees(), 9);  // 11 - 2 = 9
    assert_eq!(parser.num_total_trees(), 11);
}

#[test]
fn test_skip_first_and_burnin() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    let parser = NexusParserBuilder::for_file(path).unwrap()
        .with_skip_first()
        .with_burnin(Burnin::Count(2))
        .eager()
        .build()
        .unwrap();

    // Skip first (1), then burnin from remaining (2), so 11 - 1 - 2 = 8
    assert_eq!(parser.num_trees(), 8);
    assert_eq!(parser.num_total_trees(), 11);
}

#[test]
fn test_lazy_mode() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    let mut parser = NexusParserBuilder::for_file(path).unwrap()
        .lazy()
        .build()
        .unwrap();

    assert_eq!(parser.num_trees(), 11);

    // Parse all trees in lazy mode
    let mut count = 0;
    while let Some(tree) = parser.next_tree().unwrap() {
        assert_eq!(tree.num_leaves(), 20);
        assert!(tree.is_valid());
        count += 1;
    }
    assert_eq!(count, 11);
}

#[test]
fn test_lazy_mode_with_burnin() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    let mut parser = NexusParserBuilder::for_file(path).unwrap()
        .lazy()
        .with_burnin(Burnin::Count(3))
        .build()
        .unwrap();

    assert_eq!(parser.num_trees(), 8);  // 11 - 3 = 8

    let mut count = 0;
    while let Some(tree) = parser.next_tree().unwrap() {
        assert_eq!(tree.num_leaves(), 20);
        assert!(tree.is_valid());
        count += 1;
    }
    assert_eq!(count, 8);
}

#[test]
fn test_lazy_mode_reset() {
    let path = Path::new("tests").join("fixtures").join("nexus_t3_n10_comments.trees");

    let mut parser = NexusParserBuilder::for_file(path).unwrap()
        .lazy()
        .build()
        .unwrap();

    // Parse first tree
    let tree1 = parser.next_tree().unwrap().unwrap();
    assert_eq!(tree1.num_leaves(), 10);

    // Reset and parse again
    parser.reset();
    let tree1_again = parser.next_tree().unwrap().unwrap();
    assert_eq!(tree1_again.num_leaves(), 10);

    // Trees should be equivalent (same structure)
    assert_eq!(tree1.to_newick(&NewickStyle::ZeroIndexed, None),
        tree1_again.to_newick(&NewickStyle::ZeroIndexed, None));
}

#[test]
fn test_lazy_mode_reset_with_burnin() {
    let path = Path::new("tests").join("fixtures").join("nexus_t11_n20_translate.trees");

    let mut parser = NexusParserBuilder::for_file(path).unwrap()
        .lazy()
        .with_skip_first()
        .with_burnin(Burnin::Count(2))
        .build()
        .unwrap();

    // Should start at tree index 3 (skip 1, burnin 2)
    assert_eq!(parser.num_trees(), 8);

    // Parse first available tree
    let first_tree = parser.next_tree().unwrap().unwrap();

    // Parse a few more
    parser.next_tree().unwrap();
    parser.next_tree().unwrap();

    // Reset back to beginning (after skip+burnin)
    parser.reset();
    let first_tree_again = parser.next_tree().unwrap().unwrap();

    // Should get the same first tree again
    assert_eq!(first_tree.to_newick(&NewickStyle::ZeroIndexed, None),
        first_tree_again.to_newick(&NewickStyle::ZeroIndexed, None));
}

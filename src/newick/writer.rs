//! Newick format file writing for

use crate::parser::utils::escape_label;
use crate::model::leaf_label_map::LeafLabelMap;
use crate::model::CompactTree;
use crate::model::tree::VertexIndex;
use crate::model::vertex::BranchLength;
use std::fs::File;
use std::io::{self, BufWriter, Write};

/// Extra buffer in Newick string length/capacity estimate
const BUFFER_CHARS: usize = 10;

/// Style for serializing tree to Newick format,
/// controlling how leaf labels are represented in the output string.
#[derive(Debug, Clone, Copy)]
pub enum NewickStyle {
    /// Use full leaf labels from the LeafLabelMap
    Label,
    /// Use 0-based indices (0, 1, 2, ...)
    ZeroIndexed,
    /// Use 1-based indices (1, 2, 3, ...) (as in Nexus files)
    OneIndexed,
}

/// Writes given list of trees to a file in Newick format, one tree per line.
///
/// Each tree is written as a complete Newick string followed by a newline.
/// Label names are based on the given leaf label lab and escaped if necessary.
///
/// # Arguments
/// * `file` - The file to write to
/// * `trees` - Vector of trees to write
/// * `leaf_label_map` - Shared leaf label mapping for all trees
///
/// # Errors
/// Returns an I/O error if writing fails.
///
/// # Example
/// ```ignore
/// use nexwick::newick::write_newick_file;
/// use std::fs::File;
///
/// let file = File::create("trees.nwk")?;
/// write_newick_file(file, &your_trees, &your_label_map)?;
/// ```
pub fn write_newick_file(file: File, trees: &[CompactTree], leaf_label_map: Option<&LeafLabelMap>) -> io::Result<()> {
    if trees.len() == 0 {
        return Ok(());
    }

    let mut writer = BufWriter::new(file);
    let estimated_capacity = estimate_newick_len(&NewickStyle::Label, trees.first().unwrap(), leaf_label_map);
    for tree in trees {
        let newick = to_newick_with_capacity(&NewickStyle::Label, tree, leaf_label_map, estimated_capacity);
        writer.write_all(newick.as_bytes())?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

/// Returns the Newick representation of this tree with closing semicolon.
///
/// The Newick format represents phylogenetic trees as nested parentheses with branch lengths.
/// For example: `(('Little Spotted Kiwi;'1.0,'Great Spotted Kiwi':1.0):0.5,'Okarito Brown Kiwi':1.5);`
///
/// # Arguments
/// * `style` - The [NewickStyle] used to represent leaf labels in the output
/// * `leaf_label_map` - [Mapping](LeafLabelMap) required when using [NewickStyle::Label], otherwise can be `None`
///
/// # Returns
/// A Newick format string terminated with `;`. Returns an empty string if
/// `NewickStyle::Label` is used without providing a [LeafLabelMap].
///
/// # Example
/// ```
/// use nexwick::newick::NewickStyle;
/// use nexwick::model::tree::GenTree;
/// use nexwick::model::leaf_label_map::LeafLabelMap;
/// use nexwick::model::vertex::BranchLength;
///
/// let mut tree = GenTree::new(2);
/// let mut labels = LeafLabelMap::new(2);
/// let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
/// let b = tree.add_leaf(Some(BranchLength::new(2.0)), labels.get_or_insert("B"));
/// tree.add_root_without_branch((a, b));
///
/// let newick = tree.to_newick(&NewickStyle::Label, Some(&labels));
/// assert_eq!(newick, "(A:1,B:2);");
/// ```
pub fn to_newick(style: &NewickStyle, tree: &CompactTree, leaf_label_map: Option<&LeafLabelMap>) -> String {
    // Abort right away if arguments don't match
    if matches!(style, NewickStyle::Label) && leaf_label_map.is_none() {
        return String::new();
    }

    let estimated_capacity = estimate_newick_len(style, tree, leaf_label_map);
    to_newick_with_capacity(style, tree, leaf_label_map, estimated_capacity)
}

/// Returns the Newick representation of a tree with pre-allocated capacity.
///
/// This is an optimization for writing multiple trees with similar structure,
/// where the capacity can be estimated once and reused.
///
/// # Arguments
/// * `style` - The [NewickStyle] used to represent leaf labels in the output
/// * `tree` - The [GenTree] to convert
/// * `leaf_label_map` - [Mapping](LeafLabelMap) required when using [NewickStyle::Label], otherwise can be `None`
/// * `estimated_capacity` - Pre-estimated string capacity/len to avoid reallocations
///
/// # Returns
/// A Newick format string terminated with `;`
pub(crate) fn to_newick_with_capacity(style: &NewickStyle, tree: &CompactTree, leaf_label_map: Option<&LeafLabelMap>, estimated_capacity: usize) -> String {
    // Helper for adding branch lengths
    fn build_newick_branch_length(newick: &mut String, branch_length: Option<BranchLength>) {
        if let Some(branch_length) = branch_length {
            newick.push(':');
            newick.push_str(&branch_length.to_string());
        }
    }

    // Recursive helper for building the Newick string
    fn build_newick(tree: &CompactTree, newick: &mut String, index: VertexIndex, style: &NewickStyle, leaf_label_map: Option<&LeafLabelMap>) {
        let vertex = &tree[index];

        if vertex.is_leaf() {
            // Add label based on style
            let label_index = vertex.label().unwrap();
            match style {
                NewickStyle::Label => {
                    let label = &leaf_label_map.unwrap()[*label_index];
                    let escaped = escape_label(label);
                    newick.push_str(&escaped);
                }
                NewickStyle::ZeroIndexed => {
                    newick.push_str(&label_index.to_string());
                }
                NewickStyle::OneIndexed => {
                    newick.push_str(&(label_index + 1).to_string());
                }
            }
            build_newick_branch_length(newick, vertex.branch_length());
        } else {
            let (left, right) = vertex.children().unwrap();

            newick.push('(');
            build_newick(tree, newick, left, style, leaf_label_map);
            newick.push(',');
            build_newick(tree, newick, right, style, leaf_label_map);
            newick.push(')');

            if !vertex.is_root() {
                build_newick_branch_length(newick, vertex.branch_length());
            }
        }
    }

    let mut newick = String::with_capacity(estimated_capacity);

    build_newick(&tree, &mut newick, tree.root_index(), &style, leaf_label_map);
    newick.push(';');

    newick
}

/// Estimates the length of a Newick string for a given tree.
///
/// This function calculates the expected number of characters needed to represent
/// a tree in Newick format, accounting for structure, labels/indices, and branch lengths.
/// The estimate is used to pre-allocate string capacity for efficient writing.
///
/// # Arguments
/// * `style` - The [NewickStyle] used to represent leaf labels in the output
/// * `tree` - The [GenTree] to estimate length for
/// * `leaf_label_map` - [Mapping](LeafLabelMap) required when using [NewickStyle::Label], otherwise can be `None`
///
/// # Returns
/// Estimated number of characters needed for the Newick representation
pub(crate) fn estimate_newick_len(style: &NewickStyle, tree: &CompactTree, leaf_label_map: Option<&LeafLabelMap>) -> usize {
    // Each internal node: "(,)" ~= 3 chars
    const INTERNAL_NODE_CHARS: usize = 3;  // "(,)"
    // Branch lengths: ~20 chars each (e.g., ":0.009529961339106089")
    const BRANCH_LENGTH_CHARS: usize = 20;

    // -> Structural
    let num_internal = tree.num_internal() + 1; // +1 for root
    let structure_capacity = num_internal * INTERNAL_NODE_CHARS;

    // -> Labels
    let num_leaves = tree.num_leaves();
    let label_capacity = match style {
        NewickStyle::Label => {
            let total_label_len: usize = leaf_label_map.unwrap()
                .labels()
                .iter()
                .map(|s| escape_label(s).len())
                .sum();
            total_label_len
        }
        NewickStyle::ZeroIndexed => {
            calculate_index_digit_capacity(num_leaves, true)
        }
        NewickStyle::OneIndexed => {
            calculate_index_digit_capacity(num_leaves, false)
        }
    };

    // -> Branch lengths
    let branch_capacity = if tree.vertices_have_branch_lengths() {
        (num_leaves + num_internal - 1) * BRANCH_LENGTH_CHARS
    } else {
        0
    };

    // => Total

    let estimated_capacity = structure_capacity + label_capacity + branch_capacity + BUFFER_CHARS;

    estimated_capacity
}

/// Calculates the total number of characters needed to represent all indices.
///
/// This function efficiently computes the sum of digits for a range of indices
/// using the formula: `count * max_digits - overcounting_adjustment`
///
/// # Arguments
/// * `count` - The number of indices (number of leaves/taxa)
/// * `zero_indexed` - Whether indices start at 0 (true) or 1 (false)
///
/// # Returns
/// Total number of characters needed for all indices
///
/// # Examples
/// - 14 leaves, 1-indexed (1-14): 14*2 - 9 = 19 chars
/// - 102 leaves, 1-indexed (1-102): 102*3 - 9 - 90 = 198 chars
/// - 10 leaves, 0-indexed (0-9): 10*1 = 10 chars
fn calculate_index_digit_capacity(count: usize, zero_indexed: bool) -> usize {
    if count == 0 {
        return 0;
    }

    let max_index = if zero_indexed { count - 1 } else { count };

    if max_index == 0 {
        return 1; // Just "0"
    }

    // Number of digits in max_index
    let max_digits = (max_index as f64).log10().floor() as usize + 1;

    // Step 1: Overestimate (as if all indices had max_digits)
    let mut total = max_index * max_digits;

    // Step 2: Subtract overcounting for lower digit counts
    // For 1-indexed: subtract 9, then 99, then 999, etc.
    let mut cumulative_count = 9;
    for digits in 1..max_digits {
        total -= cumulative_count * (max_digits - digits);
        cumulative_count = cumulative_count * 10 + 9; // 9 → 99 → 999 → ...
    }

    // Step 3: Adjust for ZeroIndexed (has one extra 1-digit number: the 0)
    if zero_indexed {
        total += 1;
    }

    total
}
//! Tree module for phylogenetic tree representation.
//!
//! This module provides the core data structures for representing phylogenetic trees:
//! - [Tree]: The main tree structure using the arena pattern for efficient memory layout.
//! - [TreeIndex] is used to index vertices.
//! - [LeafLabelMap]: Joined storage and lookup for leaf labels for trees on same labels.
//! - [LabelIndex] is used to index labels.

use crate::model::vertex::{BranchLength, Vertex};
use std::collections::HashMap;
use std::fmt;

/// Index of a vertex in a tree (arena).
pub type TreeIndex = usize;

/// *During construction only*, index for unset root.
const NO_ROOT_SET_INDEX: TreeIndex = usize::MAX;

/// Index of a leaf label in a [LeafLabelMap].
pub type LabelIndex = usize;

/// A binary phylogenetic tree represented using the arena pattern on [Vertex].
///
/// Vertices are stored in a contiguous vector and referenced by [TreeIndex].
/// Aim is to avoid referencing troubles as well as to provide efficient memory layout
/// and cache locality for traversal operations.
///
/// # Structure
/// - All vertices (root, internal, and leaves) are stored in the arena
/// - Index of root is maintained
/// - No assumption on order of indices is maintained (e.g. leaves must not be first `n` indices)
/// - Leaves contain a [LabelIndex] pointing into a shared [LeafLabelMap]
/// - Branch lengths are optional, but if provided must be non-negative
///
/// # Construction
/// To construct a tree, specify its size based on the number of leaves, then add vertices one by one.
/// Bottom-up construction is likely easiest, but indices can also be managed otherwise.
/// Test validity with [Tree::is_valid].
///
/// # Example
/// ```
/// use nexus_parser::model::tree::{Tree, LeafLabelMap};
/// use nexus_parser::model::vertex::{BranchLength, Vertex};
///
/// // Create a tree: ((A:0.2,B:0.2):0.2,C:0.4):0.0;
/// let num_leaves = 3;
/// let mut tree = Tree::new(num_leaves);
/// let mut labels = LeafLabelMap::new(num_leaves);
///
/// // Add leaves (bottom-up construction)
/// let index_a = tree.add_leaf(Some(BranchLength::new(0.2)), labels.get_or_insert("A"));
/// let index_b = tree.add_leaf(Some(BranchLength::new(0.2)), labels.get_or_insert("B"));
/// let index_c = tree.add_leaf(Some(BranchLength::new(0.4)), labels.get_or_insert("C"));
///
/// // Add internal vertex with A and B as children
/// let index_internal = tree.add_internal_vertex((index_a, index_b), Some(BranchLength::new(0.2)));
///
/// // Add root with internal node and C as children
/// tree.add_root((index_internal, index_c));
///
/// assert!(tree.is_valid());
/// ```
#[derive(Debug, Clone)]
pub struct Tree {
    /// Number of leaf nodes in the tree
    num_leaves_init: usize,

    /// Vertices of this tree (arena pattern)
    vertices: Vec<Vertex>, // arena pattern

    /// Index of the root of this tree
    root_index: TreeIndex,

    /// Name of tree; optional, e.g. when parsed from Nexus file
    name: Option<String>,
}

impl Tree {
    /// Creates a new tree with capacity for a binary tree with `num_leaves` leaves.
    ///
    /// # Arguments
    /// `num_leaves` - number of leaves of the new binary tree, implying number of vertices; must be positive
    pub fn new(num_leaves: usize) -> Self {
        assert!(num_leaves > 0);
        let capacity = 2 * num_leaves - 1;
        Tree {
            num_leaves_init: num_leaves,
            name: None,
            root_index: NO_ROOT_SET_INDEX,
            vertices: Vec::with_capacity(capacity),
        }
    }

    /// Attaches a name to this tree.
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Adds a root to the tree, assigning a unique index, which gets returned.
    ///
    /// # Arguments
    /// * `children` - Tuple of child indices
    ///
    /// # Returns
    /// The index of the newly created root vertex.
    pub fn add_root(&mut self, children: (TreeIndex, TreeIndex)) -> TreeIndex {
        let index = self.vertices.len();
        self.vertices.push(Vertex::new_root(index, children));

        self.root_index = index;
        self[children.0].set_parent(index);
        self[children.1].set_parent(index);

        index
    }

    /// Adds an internal vertex to the tree, assigning a unique index, which gets returned.
    ///
    /// # Arguments
    /// * `children` - Tuple of child indices
    /// * `branch_length` - Length of incoming branch, i.e. distance to parent (non-negative)
    ///
    /// # Returns
    /// The index of the newly created internal vertex.
    ///
    /// # Panics
    /// Panics if `branch_length` is negative.
    pub fn add_internal_vertex(&mut self, children: (TreeIndex, TreeIndex), branch_length: Option<BranchLength>) -> TreeIndex {
        let index = self.vertices.len();
        self.vertices.push(Vertex::new_internal(index, children, branch_length));

        self[children.0].set_parent(index);
        self[children.1].set_parent(index);

        index
    }

    /// Adds a leaf to the tree, assigning a unique index, which gets returned.
    ///
    /// # Arguments
    /// * `branch_length` - Length of incoming branch, i.e. distance to parent (non-negative)
    /// * `label_index` - Index into the leaf label map for this leaf's name
    ///
    /// # Returns
    /// The index of the newly created leaf vertex.
    ///
    /// # Panics
    /// Panics if `branch_length` is negative.
    pub fn add_leaf(&mut self, branch_length: Option<BranchLength>, label_index: LabelIndex) -> usize {
        let index = self.vertices.len();
        self.vertices.push(Vertex::new_leaf(index, branch_length, label_index));
        index
    }

    /// Validates the tree structure and all index references.
    ///
    /// Checks:
    /// - Root index is valid and points to a Root vertex
    /// - All vertex indices match their position in the arena
    /// - There are the right number of leaves and only one root
    /// - All child indices are valid and point back to correct parent
    /// - All parent indices are valid and include this vertex as a child
    /// - Root vertex has no parent set, all others have valid parent set
    ///
    /// # Returns
    /// `true` if tree is valid, `false` otherwise
    pub fn is_valid(&self) -> bool {
        // Check root index is set
        if self.root_index == NO_ROOT_SET_INDEX {
            return false;
        }

        // Check root index is within bounds
        if self.root_index >= self.vertices.len() {
            return false;
        }

        // Check root is actually a Root variant
        if !self.vertices[self.root_index].is_root() {
            return false;
        }

        let mut leaf_count = 0;
        let mut found_root = false;

        // Validate each vertex
        for (index, vertex) in self.vertices.iter().enumerate() {
            // Check vertex index matches its arena position
            if vertex.index() != index {
                return false;
            }

            // Check that there is only one root
            if vertex.is_root() {
                if found_root {
                    return false;
                } else {
                    found_root = true;
                }
            }

            // Check that there are not too many leaves
            if vertex.is_leaf() {
                leaf_count += 1;
                if leaf_count > self.num_leaves_init {
                    return false;
                }
            }

            // Check children references
            if let Some((left, right)) = vertex.children() {
                // Check child indices are in bounds
                if left >= self.vertices.len() || right >= self.vertices.len() {
                    return false;
                }

                // Check children point back to this vertex as parent
                let left_parent = self.vertices[left].parent_index();
                let right_parent = self.vertices[right].parent_index();

                if left_parent != Some(index) || right_parent != Some(index) {
                    return false;
                }
            }

            // Check parent references
            if vertex.is_root() {
                // Root should not have a parent set
                if vertex.has_parent() {
                    return false;
                }
            } else {
                // Non-root must have valid parent
                match vertex.parent_index() {
                    None => return false, // Non-root without parent
                    Some(parent_index) => {
                        // Check parent index is in bounds
                        if parent_index >= self.vertices.len() {
                            return false;
                        }

                        // Check parent includes this vertex in its children
                        if let Some((left, right)) = self.vertices[parent_index].children() {
                            if left != index && right != index {
                                return false;
                            }
                        } else {
                            // Parent has no children - invalid
                            return false;
                        }
                    }
                }
            }

            // Check leaves have valid label indices
            if vertex.is_leaf() {
                let label_index = vertex.label_index();
                if label_index.is_none_or(|idx| idx >= self.num_leaves_init) {
                    return false;
                }
            }
        }

        // Check that there are enough leaves
        if leaf_count < self.num_leaves_init {
            return false;
        }

        true
    }

    /// Returns reference to name of this tree, or `None` if not set.
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    /// Returns whether root of tree has been set.
    pub fn is_root_set(&self) -> bool {
        self.root_index != NO_ROOT_SET_INDEX
    }

    /// Returns a reference to the root vertex.
    ///
    /// # Panics
    /// Panics if the root hasn't been set and thus tree hasn't been fully constructed yet.
    pub fn root(&self) -> &Vertex {
        &self[self.root_index]
    }

    /// Returns a mutable reference to the root vertex.
    ///
    /// # Panics
    /// Panics if the root hasn't been set and thus tree hasn't been fully constructed yet.
    pub fn root_mut(&mut self) -> &mut Vertex {
        &mut self.vertices[self.root_index]
    }

    /// Returns a reference to the vertex at the given index.
    ///
    /// # Arguments
    /// * `index` - The index of the vertex to retrieve
    ///
    /// `Some(&Vertex)` if the index is valid, `None` otherwise
    pub fn vertex(&self, index: usize) -> &Vertex {
        &self[index]
    }

    /// Returns the number of leaves this tree was initialized to hold.
    ///
    /// This represents the capacity, not necessarily the current count of leaf vertices.
    pub fn num_leaves_init(&self) -> usize { self.num_leaves_init }

    /// Returns the number of leaves in this tree.
    pub fn num_leaves(&self) -> usize {
        self.vertices.iter().filter(|&v| v.is_leaf()).count()
    }

    /// Returns the number of leaves this tree was initialized to hold.
    pub fn num_internal(&self) -> usize {
        self.vertices.iter().filter(|&v| v.is_internal()).count()
    }

    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Prints a visual representation of the tree to the console.
    ///
    /// # Arguments
    /// * `label_map` - Optional label map to show leaf names
    ///
    /// # Example Output
    /// ```text
    /// Tree with 3 leaves (5 vertices total):
    /// Root: vertex 4
    ///   [4] Internal (branch: 0.5)
    ///     ├─ [2] Internal (branch: 0.3)
    ///     │   ├─ [0] Leaf "A" (branch: 0.1)
    ///     │   └─ [1] Leaf "B" (branch: 0.2)
    ///     └─ [3] Leaf "C" (branch: 0.4)
    /// ```
    pub fn print_tree(&self, label_map: Option<&LeafLabelMap>) {
        println!("Tree with {} leaves ({} vertices total):",
                 self.num_leaves_init, self.vertices.len());

        if self.root_index != NO_ROOT_SET_INDEX {
            println!("Root: vertex {}", self.root_index);
            self.print_vertex(self.root_index, "", true, label_map);
        } else {
            println!("(No root set)");
        }
    }

    /// Helper function to recursively print a vertex and its children.
    fn print_vertex(&self, idx: usize, prefix: &str, is_last: bool, label_map: Option<&LeafLabelMap>) {
        let vertex = &self.vertices[idx];

        // Print the current vertex
        let connector = if prefix.is_empty() { "" } else if is_last { "└─ " } else { "├─ " };

        if vertex.is_leaf() {
            let label = if let Some(map) = label_map {
                if let Some(label_idx) = vertex.label_index() {
                    map.get_label(label_idx).unwrap_or("?")
                } else {
                    "?"
                }
            } else {
                "?"
            };

            let branch_str = if let Some(bl) = vertex.branch_length() {
                format!("(branch: {:.3})", *bl)
            } else {
                "(no branch)".to_string()
            };

            println!("{}{}[{}] Leaf \"{}\" {}", prefix, connector, idx, label, branch_str);
        } else {
            let branch_str = if let Some(bl) = vertex.branch_length() {
                format!("(branch: {:.3})", *bl)
            } else {
                "(no branch)".to_string()
            };

            println!("{}{}[{}] Internal {}", prefix, connector, idx, branch_str);

            // Print children if they exist
            if let Some((left, right)) = vertex.children() {
                let new_prefix = if prefix.is_empty() {
                    "  ".to_string()
                } else {
                    format!("{}{}  ", prefix, if is_last { " " } else { "│" })
                };

                self.print_vertex(left, &new_prefix, false, label_map);
                self.print_vertex(right, &new_prefix, true, label_map);
            }
        }
    }
}

impl std::ops::Index<TreeIndex> for Tree {
    type Output = Vertex;

    fn index(&self, index: TreeIndex) -> &Self::Output {
        &self.vertices[index]
    }
}

impl std::ops::IndexMut<TreeIndex> for Tree {
    fn index_mut(&mut self, index: TreeIndex) -> &mut Self::Output {
        &mut self.vertices[index]
    }
}

/// Maps leaf labels (strings) to compact indices for efficient storage.
///
/// This bidirectional mapping allows multiple trees with the same taxa to share
/// a single label storage, with each leaf referencing labels by [LabelIndex].
/// Labels are deduplicated automatically - inserting the same label twice returns
/// the same index.
///
/// # Example
/// ```
/// use nexus_parser::model::tree::LeafLabelMap;
///
/// let mut labels = LeafLabelMap::new(3);
///
/// let idx_a = labels.get_or_insert("A");  // idx_a = 0
/// let idx_b = labels.get_or_insert("B");  // idx_b = 1
/// let idx_a2 = labels.get_or_insert("A"); // idx_a2 = 0 (deduplicated)
///
/// assert_eq!(idx_a, idx_a2);
/// assert_eq!(labels.get_label(idx_a), Some("A"));
/// ```
#[derive(Debug, Clone)]
pub struct LeafLabelMap {
    /// Expected number of unique labels
    num_leaves: usize,
    /// List of unique labels
    labels: Vec<String>,
    /// Map from label to its index
    map: HashMap<String, usize>,
}

impl LeafLabelMap {
    /// Creates a new LeafLabelMap with pre-allocated capacity.
    ///
    /// # Arguments
    /// * `num_leaves` - Expected number of unique leaf labels
    pub fn new(num_leaves: usize) -> Self {
        LeafLabelMap {
            num_leaves,
            labels: Vec::with_capacity(num_leaves),
            map: HashMap::with_capacity(num_leaves),
        }
    }

    /// Inserts a label without checking for duplicates.
    ///
    /// **Warning**: This will create duplicate entries if the label already exists.
    /// Prefer [get_or_insert] which handles deduplication.
    ///
    /// # Arguments
    /// * `label` - The label to insert
    pub fn insert(&mut self, label: String) {
        let idx = self.labels.len();
        self.labels.push(label.clone());
        self.map.insert(label, idx);
    }

    /// Gets the index for a label, inserting it if it doesn't exist.
    ///
    /// If the label already exists, returns its existing index.
    /// If the label is new, assigns it the next available index.
    ///
    /// # Arguments
    /// * `s` - The label string to look up or insert
    ///
    /// # Returns
    /// The index associated with this label
    pub fn get_or_insert(&mut self, s: &str) -> usize {
        if let Some(&index) = self.map.get(s) {
            index
        } else {
            let idx = self.labels.len();
            self.labels.push(s.to_string());
            self.map.insert(s.to_string(), idx);

            // Should not add more labels than specified by capacity `num_leaves`
            debug_assert!(idx < self.num_leaves);

            idx
        }
    }

    /// Retrieves the index for a given label.
    ///
    /// # Arguments
    /// * `label` - The label string to look up
    ///
    /// # Returns
    /// `Some(index)` if the label exists, `None` otherwise
    pub fn get_index(&self, s: &str) -> Option<LabelIndex> {
        self.map.get(s).map(|&index| index)
    }

    /// Retrieves the leaf label for a given index.
    ///
    /// # Arguments
    /// * `index` - The index to look up
    ///
    /// # Returns
    /// `Some(&str)` if the index is valid, `None` otherwise
    pub fn get_label(&self, index: usize) -> Option<&str> {
        self.labels.get(index).map(|s| s.as_str())
    }

    /// Checks if a label exists in the map.
    ///
    /// # Arguments
    /// * `label` - The label string to check
    ///
    /// # Returns
    /// `true` if the label exists, `false` otherwise
    pub fn contains_label(&self, label: &str) -> bool {
        self.map.contains_key(label)
    }

    /// Returns the number of labels currently stored.
    pub fn num_labels(&self) -> usize {
        self.labels.len()
    }

    /// Returns whether the map has reached its expected capacity.
    pub fn is_full(&self) -> bool {
        self.num_leaves == self.map.len()
    }

    /// Checks whether the given HashMap is consistent with this map:
    /// - Same length
    /// - All labels in `translation` appear in this map
    ///
    ///# Arguments
    ///* `translation` - Translation map (likely from Nexus TRANSLATE command) to test,
    ///                  with leaf labels being the map's values
    pub fn check_consistency_with_translation(&self, translation: &HashMap<String, String>) -> bool {
        // Need to have same number of labels
        if translation.len() != self.num_labels() {
            return false;
        }
        // Each label in map needs to appear
        for test_label in translation.values() {
            if !self.contains_label(test_label) {
                return false;
            }
        }

        true
    }
}

impl fmt::Display for LeafLabelMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "LeafLabelMap ({}/{} labels):", self.labels.len(), self.num_leaves)?;
        for (idx, label) in self.labels.iter().enumerate() {
            writeln!(f, "  [{}] {}", idx, label)?;
        }
        Ok(())
    }
}

impl std::ops::Index<LabelIndex> for LeafLabelMap {
    type Output = str;

    fn index(&self, index: LabelIndex) -> &Self::Output {
        &self.labels[index]
    }
}
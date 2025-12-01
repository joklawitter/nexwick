//! Tree module for phylogenetic tree representation.
//!
//! This module provides the core data structures for representing phylogenetic trees:
//! - `Tree`: The main tree structure using the arena pattern for efficient memory layout.
//! - `TreeIndex` is used to index vertices.
//! - `LabelIndex` is used to index labels.

use crate::model::leaf_label_map::LeafLabelMap;
use crate::model::vertex::{BranchLength, Vertex};

/// Float comparison tolerance
const EPSILON: f64 = 1e-7;
// Consider using relative epsilon/comparison, e.g.:
// (d1 - d2).abs < abs_tol.max(rel_tol * d1.max(d2))

/// Index of a vertex in a tree (arena).
pub type TreeIndex = usize;

/// *During construction only*, index for unset root.
const NO_ROOT_SET_INDEX: TreeIndex = usize::MAX;

/// Index of a leaf label in a [LeafLabelMap].
pub type LabelIndex = usize;


// =#========================================================================#=
// TREE
// =#========================================================================#=
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
/// use nexus_parser::model::tree::Tree;
/// use nexus_parser::model::leaf_label_map::LeafLabelMap;
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

// ============================================================================
// New, Getters / Accessors, etc. (pub)
// ============================================================================
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

    /// Returns a reference to the vertex at the given index.
    ///
    /// # Arguments
    /// * `index` - The index of the vertex to retrieve
    ///
    /// `Some(&Vertex)` if the index is valid, `None` otherwise
    pub fn vertex_mut(&mut self, index: usize) -> &mut Vertex {
        &mut self.vertices[index]
    }

    /// Returns the number of leaves this tree was initialized to hold.
    ///
    /// This represents the capacity, not necessarily the current count of leaf vertices.
    pub fn num_leaves_init(&self) -> usize { self.num_leaves_init }

    /// Returns the number of leaves in this tree.
    pub fn num_leaves(&self) -> usize {
        self.vertices.iter().filter(|&v| v.is_leaf()).count()
    }

    /// Returns the number of internal vertices in this tree.
    pub fn num_internal(&self) -> usize {
        self.vertices.iter().filter(|&v| v.is_internal()).count()
    }

    /// Returns the number of vertices in this tree.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the height of this tree (assuming it is ultrametric; undefined otherwise),
    /// that is, the distance of the root to any/each leaf.
    pub fn height(&self) -> f64 {
        self.height_of(&self.vertices[self.root_index])
    }

    /// Returns the height of the given vertex (assuming it is ultrametric; undefined otherwise),
    /// that is, the distance of the given vertex to any/each leaf.
    pub fn height_of(&self, vertex: &Vertex) -> f64 {
        let mut height = 0.0;
        loop {
            let child_index = vertex.children().unwrap().0;
            let vertex: &Vertex = &self.vertices[child_index];
            height = height + *vertex.branch_length().unwrap();

            if vertex.is_leaf() {
                break;
            }
        }

        height
    }

    /// Checks if the tree is ultrametric (all leaves equidistant from root).
    ///
    /// # Returns
    /// `true` if all leaves are at the same distance from the root (within floating point tolerance),
    /// `false` otherwise.
    ///
    /// # Panics
    /// Panics if not all vertices (besides root) have an associated [BranchLength],
    /// which can be checked first with `vertices_have_branch_lengths()`.
    pub fn is_ultrametric(&self) -> bool {
        // Store distance from leaves in subtree to parent for each vertex
        let mut distances = vec![0.0; self.num_vertices()];

        for vertex in self.post_order_iter() {
            if vertex.is_leaf() {
                distances[vertex.index()] = *vertex.branch_length().unwrap();
            } else {
                let (left, right) = vertex.children().unwrap();
                let left_dist = distances[left];
                let right_dist = distances[right];

                if (left_dist - right_dist).abs() > EPSILON {
                    return false;
                }

                if !vertex.is_root() {
                    distances[vertex.index()] = left_dist + *vertex.branch_length().unwrap();
                }
            }
        }

        true
    }

    /// Returns the sum of all branch lengths in the tree.
    ///
    /// # Panics
    /// Panics if not all vertices (besides root) have an associated [BranchLength],
    /// which can be checked first with `vertices_have_branch_lengths()`.
    pub fn total_branch_length(&self) -> f64 {
        self.vertices.iter()
            .filter_map(|v| v.branch_length())
            .map(|bl| *bl)
            .sum::<f64>()
    }

    /// Checks if all non-root vertices have branch lengths set.
    pub fn vertices_have_branch_lengths(&self) -> bool {
        for vertex in &self.vertices {
            if !vertex.is_root() && !vertex.has_branch_length() {
                return false;
            }
        }

        true
    }

    /// Returns an iterator over the tree in post-order (children before parents).
    ///
    /// Post-order traversal visits each vertex's children before visiting the vertex itself.
    /// This is useful for computing heights, aggregating data from leaves upward, etc.
    ///
    /// # Example
    /// ```
    /// use nexus_parser::model::tree::Tree;
    /// use nexus_parser::model::leaf_label_map::LeafLabelMap;
    /// use nexus_parser::model::vertex::BranchLength;
    ///
    /// let mut tree = Tree::new(2);
    /// let mut labels = LeafLabelMap::new(2);
    /// let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    /// let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    /// tree.add_root((a, b));
    ///
    /// let indices: Vec<_> = tree.post_order_iter().map(|v| v.index()).collect();
    /// // Leaves come before root
    /// ```
    pub fn post_order_iter(&self) -> PostOrderIter<'_> {
        PostOrderIter::new(self)
    }

    /// Returns an iterator over the tree in pre-order (parents before children).
    ///
    /// Pre-order traversal visits each vertex before visiting its children.
    /// This is useful for propagating data from root to leaves.
    ///
    /// # Example
    /// ```
    /// use nexus_parser::model::tree::Tree;
    /// use nexus_parser::model::leaf_label_map::LeafLabelMap;
    /// use nexus_parser::model::vertex::BranchLength;
    ///
    /// let mut tree = Tree::new(2);
    /// let mut labels = LeafLabelMap::new(2);
    /// let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    /// let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    /// tree.add_root((a, b));
    ///
    /// let indices: Vec<_> = tree.pre_order_iter().map(|v| v.index()).collect();
    /// // Root comes before leaves
    /// ```
    pub fn pre_order_iter(&self) -> PreOrderIter<'_> {
        PreOrderIter::new(self)
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

// ============================================================================
// Printing (pub) + NEWICK STYLE
// ============================================================================
impl Tree {
    /// Converts the tree to Newick format string.
    ///
    /// The Newick format represents phylogenetic trees as nested parentheses with branch lengths.
    /// For example: `(('Little Spotted Kiwi;'1.0,'Great Spotted Kiwi':1.0):0.5,'Okarito Brown Kiwi':1.5);`
    ///
    /// # Arguments
    /// * `style` - How to represent leaf labels in the output
    /// * `leaf_label_map` - Required when using `NewickStyle::Label`, otherwise can be `None`
    ///
    /// # Returns
    /// A Newick format string terminated with `;`. Returns an empty string if
    /// `NewickStyle::Label` is used without providing a [LeafLabelMap].
    ///
    /// # Example
    /// ```
    /// use nexus_parser::model::tree::{Tree, NewickStyle};
    /// use nexus_parser::model::leaf_label_map::LeafLabelMap;
    /// use nexus_parser::model::vertex::BranchLength;
    ///
    /// let mut tree = Tree::new(2);
    /// let mut labels = LeafLabelMap::new(2);
    /// let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    /// let b = tree.add_leaf(Some(BranchLength::new(2.0)), labels.get_or_insert("B"));
    /// tree.add_root((a, b));
    ///
    /// let newick = tree.to_newick(NewickStyle::Label, Some(&labels));
    /// assert_eq!(newick, "(A:1,B:2);");
    /// ```
    pub fn to_newick(&self, style: NewickStyle, leaf_label_map: Option<&LeafLabelMap>) -> String {
        // Helper for adding branch lengths
        fn build_newick_branch_length(newick: &mut String, branch_length: Option<BranchLength>) {
            if let Some(branch_length) = branch_length {
                newick.push(':');
                newick.push_str(&branch_length.to_string());
            }
        }

        // Recursive helper for building the Newick string
        fn build_newick(tree: &Tree, newick: &mut String, index: TreeIndex, style: &NewickStyle, leaf_label_map: Option<&LeafLabelMap>) {
            let vertex = &tree[index];

            if vertex.is_leaf() {
                // Add label based on style
                let label_index = vertex.label_index().unwrap();
                match style {
                    NewickStyle::Label => {
                        let label = &leaf_label_map.unwrap()[label_index];
                        newick.push_str(label);
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

        // Abort right away if arguments don't match
        if matches!(style, NewickStyle::Label) && leaf_label_map.is_none() {
            return String::new();
        }

        // Estimate capacity:
        // - Each leaf: "label" (can compute total) or "id" ~= 2
        const LEAF_ID_CHARS: usize = 2;  // "99" for indices
        // - Each internal node: "(,)" ~= 3 chars
        const INTERNAL_NODE_CHARS: usize = 3;  // "(,)"
        // - Branch lengths: ~20 chars each (e.g., ":0.009529961339106089")
        const BRANCH_LENGTH_CHARS: usize = 20;

        // -> Structural
        let num_internal = self.num_internal() + 1; // +1 for root
        let structure_capacity = num_internal * INTERNAL_NODE_CHARS;

        // -> Labels
        let num_leaves = self.num_leaves();
        let label_capacity = match style {
            NewickStyle::Label => {
                let total_label_len: usize = leaf_label_map.unwrap().labels().iter().map(|s| s.len()).sum();
                total_label_len
            }
            NewickStyle::ZeroIndexed | NewickStyle::OneIndexed => {
                num_leaves * LEAF_ID_CHARS
            }
        };

        // -> Branch lengths
        let branch_capacity = if self.vertices_have_branch_lengths() {
            (num_leaves + num_internal - 1) * BRANCH_LENGTH_CHARS
        } else {
            0
        };

        // => Total
        let estimated_capacity = structure_capacity + label_capacity + branch_capacity;
        let mut newick = String::with_capacity(estimated_capacity);

        build_newick(&self, &mut newick, self.root_index, &style, leaf_label_map);
        newick.push(';');

        newick
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


// =#========================================================================#=
// ITERATORS
// =#========================================================================#=
/// Iterator for post-order traversal (children before parents).
///
/// This iterator uses a stack-based approach to traverse the tree without recursion.
/// Each vertex is visited after all its descendants have been visited.
pub struct PostOrderIter<'a> {
    tree: &'a Tree,
    stack: Vec<(TreeIndex, bool)>, // (index, children_visited)
}

impl<'a> PostOrderIter<'a> {
    fn new(tree: &'a Tree) -> Self {
        let mut stack = Vec::new();
        if tree.is_root_set() {
            stack.push((tree.root_index, false));
        }
        PostOrderIter { tree, stack }
    }
}

impl<'a> Iterator for PostOrderIter<'a> {
    type Item = &'a Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((index, children_visited)) = self.stack.pop() {
            let vertex = &self.tree[index];

            if children_visited || vertex.is_leaf() {
                // Either we've already processed children, or this is a leaf
                return Some(vertex);
            } else {
                // Mark this vertex as "children will be visited"
                self.stack.push((index, true));

                // Push children (right first, so left is processed first)
                if let Some((left, right)) = vertex.children() {
                    self.stack.push((right, false));
                    self.stack.push((left, false));
                }
            }
        }
        None
    }
}

/// Iterator for pre-order traversal (parents before children).
///
/// This iterator uses a stack-based approach to traverse the tree without recursion.
/// Each vertex is visited before any of its descendants.
pub struct PreOrderIter<'a> {
    tree: &'a Tree,
    stack: Vec<TreeIndex>,
}

impl<'a> PreOrderIter<'a> {
    fn new(tree: &'a Tree) -> Self {
        let mut stack = Vec::new();
        if tree.is_root_set() {
            stack.push(tree.root_index);
        }
        PreOrderIter { tree, stack }
    }
}

impl<'a> Iterator for PreOrderIter<'a> {
    type Item = &'a Vertex;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.stack.pop()?;
        let vertex = &self.tree[index];

        // Push children onto stack (right first, so left is processed first)
        if let Some((left, right)) = vertex.children() {
            self.stack.push(right);
            self.stack.push(left);
        }

        Some(vertex)
    }
}


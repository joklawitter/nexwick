//! Provides generic tree representations.
//!
//! Provides core data structures for representing phylogenetic trees:
//! * [`GenTree<LabelRef>`] - Main tree structure using the arena pattern
//!   for efficient memory layout, generic over way vertices handle labels.
//! * [CompactTree] as realization with [LabelIndex]
//! * [SimpleTree] as realization with [String]
//! * [VertexIndex] as type used to index vertices in tree

use crate::model::leaf_label_map::{LabelIndex, LeafLabelMap};
use crate::model::vertex::{BranchLength, Vertex};
use crate::newick;
use crate::newick::NewickStyle;

/// Float comparison tolerance
const EPSILON: f64 = 1e-7;
// Consider using relative epsilon/comparison, e.g.:
// (d1 - d2).abs < abs_tol.max(rel_tol * d1.max(d2))

/// Index of a vertex in a tree (arena).
pub type VertexIndex = usize;

/// *During construction only*, index for unset root.
const NO_ROOT_SET_INDEX: VertexIndex = usize::MAX;

// =$========================================================================$=
// TREE
// =$========================================================================$=
/// A binary phylogenetic tree represented using the arena pattern
/// on [Vertex].
///
/// Vertices are stored in a contiguous vector and referenced by
/// [VertexIndex]. Aim is to avoid referencing troubles as well as to provide
/// efficient memory layout and cache locality for traversal operations.
///
/// Generic over `L` (LabelRef), representing how leaves handle labels
/// (e.g. as index or String).
///
/// # Structure
/// - All vertices (root, internal, and leaves) are stored in the arena.
/// - Index of root is maintained.
/// - No assumption on order of indices is maintained.
///   (e.g. leaves must not be first `n` indices)
/// - Leaves handle labels via their label reference type `L`,
///   e.g. implementation [CompactTree] pointing into a shared [LeafLabelMap].
/// - Branch lengths are optional, but if provided must be non-negative.
///
/// # Construction
/// To construct a tree, specify its size based on the number of leaves,
/// then add vertices one by one. Bottom-up construction is likely easiest,
/// but indices can also be managed otherwise.
/// Test validity with [`GenTree::is_valid()`].
#[derive(Debug, Clone)]
pub struct GenTree<L> {
    /// Number of leaf nodes in the tree
    num_leaves_init: usize,

    /// Vertices of this tree (arena pattern)
    vertices: Vec<Vertex<L>>, // arena pattern

    /// Index of the root of this tree
    root_index: VertexIndex,

    /// Name of tree; optional, e.g. when parsed from Nexus file
    name: Option<String>,
}

// Convenient type aliases
/// Tree with shared labels via [LeafLabelMap], which is efficient for set of trees.
pub type CompactTree = GenTree<LabelIndex>;

/// Tree with embedded String labels.
pub type SimpleTree = GenTree<String>;

// ============================================================================
// New, Getters / Accessors, etc. (pub)
// ============================================================================
impl<L> GenTree<L> {
    /// Creates a new tree with capacity for a binary tree with `num_leaves` leaves.
    ///
    /// # Arguments
    /// `num_leaves` - number of leaves of the new binary tree, implying number of vertices; must be positive
    pub fn new(num_leaves: usize) -> Self {
        assert!(num_leaves > 0);
        let capacity = 2 * num_leaves - 1;
        GenTree {
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
    /// * `branch_length` - Optional length of incoming edge (for special cases, non-negative)
    ///
    /// # Returns
    /// The index of the newly created root vertex.
    pub fn add_root(
        &mut self,
        children: (VertexIndex, VertexIndex),
        branch_length: Option<BranchLength>,
    ) -> VertexIndex {
        let index = self.vertices.len();
        self.vertices
            .push(Vertex::new_root(index, children, branch_length));

        self.root_index = index;
        self[children.0].set_parent(index);
        self[children.1].set_parent(index);

        index
    }

    /// Adds a root to the tree, assigning a unique index, which gets returned.
    ///
    /// # Arguments
    /// * `children` - Tuple of child indices
    ///
    /// # Returns
    /// The index of the newly created root vertex.
    pub fn add_root_without_branch(&mut self, children: (VertexIndex, VertexIndex)) -> VertexIndex {
        self.add_root(children, None)
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
    pub fn add_internal_vertex(
        &mut self,
        children: (VertexIndex, VertexIndex),
        branch_length: Option<BranchLength>,
    ) -> VertexIndex {
        let index = self.vertices.len();
        self.vertices
            .push(Vertex::new_internal(index, children, branch_length));

        self[children.0].set_parent(index);
        self[children.1].set_parent(index);

        index
    }

    /// Adds a leaf to the tree, assigning a unique index, which gets returned.
    ///
    /// # Arguments
    /// * `branch_length` - Length of incoming branch, i.e. distance to parent (non-negative)
    /// * `label` - Label (ref) for this leaf (type depends on tree variant)
    ///
    /// # Returns
    /// The index of the newly created leaf vertex.
    ///
    /// # Panics
    /// Panics if `branch_length` is negative.
    pub fn add_leaf(&mut self, branch_length: Option<BranchLength>, label: L) -> usize {
        let index = self.vertices.len();
        self.vertices
            .push(Vertex::new_leaf(index, branch_length, label));
        index
    }

    /// Returns reference to name of this tree, or `None` if not set.
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    /// Set a name for this tree.
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Returns whether root of tree has been set.
    pub fn is_root_set(&self) -> bool {
        self.root_index != NO_ROOT_SET_INDEX
    }

    /// Returns a reference to the root vertex.
    ///
    /// # Panics
    /// Panics if the root hasn't been set and thus tree hasn't been fully constructed yet.
    pub fn root(&self) -> &Vertex<L> {
        &self[self.root_index]
    }

    /// Returns a mutable reference to the root vertex.
    ///
    /// # Panics
    /// Panics if the root hasn't been set and thus tree hasn't been fully constructed yet.
    pub fn root_mut(&mut self) -> &mut Vertex<L> {
        &mut self.vertices[self.root_index]
    }

    /// Returns the index of the root.
    pub fn root_index(&self) -> VertexIndex {
        self.root_index
    }

    /// Returns a reference to the vertex at the given index.
    ///
    /// # Arguments
    /// * `index` - The index of the vertex to retrieve
    ///
    /// # Returns
    /// `Some(&Vertex)` if the index is valid
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn vertex(&self, index: usize) -> &Vertex<L> {
        &self[index]
    }

    /// Returns a mutable reference to the vertex at the given index.
    ///
    /// # Arguments
    /// * `index` - The index of the vertex to retrieve
    ///
    /// # Returns
    /// `Some(&mut Vertex)` if the index is valid
    ///
    /// # Panics
    /// Panics if `index` is out of bounds.
    pub fn vertex_mut(&mut self, index: usize) -> &mut Vertex<L> {
        &mut self.vertices[index]
    }

    /// Returns the number of leaves this tree was initialized to hold.
    ///
    /// This represents the capacity, not necessarily the current count of leaf vertices.
    pub fn num_leaves_init(&self) -> usize {
        self.num_leaves_init
    }

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

    /// Returns the height of the given vertex (assuming it is ultrametric;
    /// result undefined otherwise), that is, the distance of the given vertex
    /// to any/each leaf.
    ///
    /// # Arguments
    /// * `vertex` - Vertex for which you want the height
    pub fn height_of(&self, vertex: &Vertex<L>) -> f64 {
        let mut height = 0.0;
        let mut current_vertex = vertex;
        loop {
            if current_vertex.is_leaf() {
                break;
            }

            let child_index = current_vertex.children().unwrap().0;
            current_vertex = &self.vertices[child_index];
            height += *current_vertex.branch_length().unwrap();
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
                let left_dist: f64 = distances[left];
                let right_dist: f64 = distances[right];

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
        self.vertices
            .iter()
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
}

impl<L: ValidLabel> GenTree<L> {
    /// Validates the tree structure and all index references.
    ///
    /// Checks:
    /// - Root index is valid and points to a Root vertex
    /// - All vertex indices match their position in the arena
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
            }

            // Check children references
            if let Some((left, right)) = vertex.children() {
                // Check child indices are in bounds
                if left >= self.vertices.len() || right >= self.vertices.len() {
                    return false;
                }

                // Check children point back to this vertex as parent
                let left_parent = self.vertices[left].parent();
                let right_parent = self.vertices[right].parent();

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
                match vertex.parent() {
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
                let label = vertex.label();
                if label.is_none_or(|l| !l.is_valid_for_tree(self.num_leaves())) {
                    return false;
                }
            }
        }

        // Check leaf count matches binary tree invariant:
        // for n leaves, there are 2n-1 vertices
        let expected_leaf_count = self.vertices.len().div_ceil(2);
        if leaf_count != expected_leaf_count {
            return false;
        }

        true
    }
}

impl<L> std::ops::Index<VertexIndex> for GenTree<L> {
    type Output = Vertex<L>;

    fn index(&self, index: VertexIndex) -> &Self::Output {
        &self.vertices[index]
    }
}

impl<L> std::ops::IndexMut<VertexIndex> for GenTree<L> {
    fn index_mut(&mut self, index: VertexIndex) -> &mut Self::Output {
        &mut self.vertices[index]
    }
}

// ============================================================================
// Printing (pub, only for CompactTree)
// ============================================================================
impl CompactTree {
    /// Convenience method to convert this tree to a Newick string
    pub fn to_newick(&self, style: &NewickStyle, leaf_label_map: Option<&LeafLabelMap>) -> String {
        newick::to_newick(style, self, leaf_label_map)
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
        println!(
            "Tree with {} leaves ({} vertices total):",
            self.num_leaves_init,
            self.vertices.len()
        );

        if self.root_index != NO_ROOT_SET_INDEX {
            println!("Root: vertex {}", self.root_index);
            self.print_vertex(self.root_index, "", true, label_map);
        } else {
            println!("(No root set)");
        }
    }

    /// Helper function to recursively print a vertex and its children.
    fn print_vertex(
        &self,
        idx: usize,
        prefix: &str,
        is_last: bool,
        label_map: Option<&LeafLabelMap>,
    ) {
        let vertex = &self.vertices[idx];

        // Print the current vertex
        let connector = if prefix.is_empty() {
            ""
        } else if is_last {
            "└─ "
        } else {
            "├─ "
        };

        if vertex.is_leaf() {
            let label = if let Some(map) = label_map {
                if let Some(label_idx) = vertex.label() {
                    map.get_label(*label_idx).unwrap_or("?")
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

            println!(
                "{}{}[{}] Leaf \"{}\" {}",
                prefix, connector, idx, label, branch_str
            );
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

// =$========================================================================$=
// ITERATORS
// =$========================================================================$=
impl<L> GenTree<L> {
    /// Returns an iterator over the tree in post-order (children before parents).
    ///
    /// Post-order traversal visits each vertex's children before visiting the vertex itself.
    /// This is useful for computing heights, aggregating data from leaves upward, etc.
    ///
    /// # Example
    /// ```
    /// use nexwick::model::tree::GenTree;
    /// use nexwick::model::leaf_label_map::LeafLabelMap;
    /// use nexwick::model::vertex::BranchLength;
    ///
    /// let mut tree = GenTree::new(2);
    /// let mut labels = LeafLabelMap::new(2);
    /// let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    /// let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    /// tree.add_root_without_branch((a, b));
    ///
    /// let indices: Vec<_> = tree.post_order_iter().map(|v| v.index()).collect();
    /// // Leaves come before root
    /// ```
    pub fn post_order_iter(&self) -> PostOrderIter<'_, L> {
        PostOrderIter::new(self)
    }

    /// Returns an iterator over the tree in pre-order (parents before children).
    ///
    /// Pre-order traversal visits each vertex before visiting its children.
    /// This is useful for propagating data from root to leaves.
    ///
    /// # Example
    /// ```
    /// use nexwick::model::tree::GenTree;
    /// use nexwick::model::leaf_label_map::LeafLabelMap;
    /// use nexwick::model::vertex::BranchLength;
    ///
    /// let mut tree = GenTree::new(2);
    /// let mut labels = LeafLabelMap::new(2);
    /// let a = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("A"));
    /// let b = tree.add_leaf(Some(BranchLength::new(1.0)), labels.get_or_insert("B"));
    /// tree.add_root_without_branch((a, b));
    ///
    /// let indices: Vec<_> = tree.pre_order_iter().map(|v| v.index()).collect();
    /// // Root comes before leaves
    /// ```
    pub fn pre_order_iter(&self) -> PreOrderIter<'_, L> {
        PreOrderIter::new(self)
    }
}

/// Iterator for post-order traversal (children before parents).
///
/// This iterator uses a stack-based approach to traverse the tree without recursion.
/// Each vertex is visited after all its descendants have been visited.
pub struct PostOrderIter<'a, L> {
    tree: &'a GenTree<L>,
    stack: Vec<(VertexIndex, bool)>, // (index, children_visited)
}

impl<'a, L> PostOrderIter<'a, L> {
    fn new(tree: &'a GenTree<L>) -> Self {
        let mut stack = Vec::new();
        if tree.is_root_set() {
            stack.push((tree.root_index, false));
        }
        PostOrderIter { tree, stack }
    }
}

impl<'a, L> Iterator for PostOrderIter<'a, L> {
    type Item = &'a Vertex<L>;

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
pub struct PreOrderIter<'a, L> {
    tree: &'a GenTree<L>,
    stack: Vec<VertexIndex>,
}

impl<'a, L> PreOrderIter<'a, L> {
    fn new(tree: &'a GenTree<L>) -> Self {
        let mut stack = Vec::new();
        if tree.is_root_set() {
            stack.push(tree.root_index);
        }
        PreOrderIter { tree, stack }
    }
}

impl<'a, L> Iterator for PreOrderIter<'a, L> {
    type Item = &'a Vertex<L>;

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

// =#========================================================================#=
// VALID LABEL TRAIT
// =#========================================================================T=
/// Trait for label types that can be validated in a tree context.
pub trait ValidLabel {
    /// Checks whether this label is valid on a very basic level,
    /// e.g. for a label index whether it is in range and a String non-empty
    fn is_valid_for_tree(&self, num_leaves: usize) -> bool;
}

impl ValidLabel for LabelIndex {
    fn is_valid_for_tree(&self, num_leaves: usize) -> bool {
        *self < num_leaves
    }
}

impl ValidLabel for String {
    fn is_valid_for_tree(&self, _num_leaves: usize) -> bool {
        !self.is_empty()
    }
}

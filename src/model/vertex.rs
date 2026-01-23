//! Vertex module for phylogenetic tree representation.
//!
//! Main component is the [Vertex] enum, coming in varieties `Root`, `Internal`, and `Leaf`,
//! and uses [BranchLength] structure to store branch/edge lengths.

use std::fmt;
use crate::model::tree::VertexIndex;
use std::ops::Deref;

/// During construction, Internal and Leaf vertex might not have parent set yet.
const NO_PARENT_SET: VertexIndex = usize::MAX;

// =#========================================================================#=
// VERTEX
// =#========================================================================€=
/// Represents a vertex (node) in a phylogenetic tree.
///
/// A vertex can be either:
/// - **Root**: Has two children, no parent, but might have branch_length (exists for special cases)
/// - **Internal**: Has two children, no label, might have branch_length
/// - **Leaf**: Has no children, has label (via index) and might have branch_length
///
/// # Invariants
/// - `index` is index in arena; non-negative (guaranteed by `TreeIndex = usize` type)
/// - `branch_length` is non-negative (enforced); might not be set
/// - Internal vertices and Leaf have `parent` is `TreeIndex` of parent in arena; `NO_PARENT_SET = usize::MAX` only during construction
/// - Internal vertices have `children` as tuple of `TreeIndex`
/// - Leaf vertices have a `label_index`, since many trees share labels
#[derive(PartialEq, Debug, Clone)]
pub enum Vertex<L> {
    /// Root vertex of the tree (has no parent, has two children)
    Root {
        /// Index of this vertex in the tree arena
        index: VertexIndex,
        /// Indices of the two child vertices
        children: (VertexIndex, VertexIndex),
        /// Optional length of incoming edge (optional and only for special cases, non-negative if present)
        branch_length: Option<BranchLength>,
    },
    /// Internal vertex (has parent and two children, no label)
    Internal {
        /// Index of this vertex in the tree arena
        index: VertexIndex,
        /// Index of the parent vertex
        parent: VertexIndex,
        /// Indices of the two child vertices
        children: (VertexIndex, VertexIndex),
        /// Distance to parent node (optional, non-negative if present)
        branch_length: Option<BranchLength>,
    },
    /// Leaf vertex (has parent and label, no children)
    Leaf {
        /// Index of this vertex in the tree arena
        index: VertexIndex,
        /// Index into the shared label map
        label: L,
        /// Index of the parent vertex
        parent: VertexIndex,
        /// Distance to parent node (optional, non-negative if present)
        branch_length: Option<BranchLength>,
    },
}

impl<L> Vertex<L> {
    /// Creates a new root vertex with optional branch length.
    ///
    /// # Arguments
    /// * `index` - The unique index of this vertex in the tree (arena)
    /// * `children` - Tuple of child indices
    /// * `branch_length` - Optional length of incoming edge (for special cases)
    pub fn new_root(index: VertexIndex, children: (VertexIndex, VertexIndex), branch_length: Option<BranchLength>) -> Self {
        Vertex::Root {
            index,
            children,
            branch_length,
        }
    }

    /// Creates a new root vertex.
    ///
    /// # Arguments
    /// * `index` - The unique index of this vertex in the tree (arena)
    /// * `children` - Tuple of child indices
    pub fn new_root_without_branch(index: VertexIndex, children: (VertexIndex, VertexIndex)) -> Self {
        Vertex::Root {
            index,
            children,
            branch_length: None,
        }
    }

    /// Creates a new internal (non-leaf, non-root) vertex .
    ///
    /// # Arguments
    /// * `index` - The unique index of this vertex in the tree (arena)
    /// * `children` - Tuple of child indices
    /// * `branch_length` - Distance to parent node (non-negative)
    pub fn new_internal(index: VertexIndex, children: (VertexIndex, VertexIndex), branch_length: Option<BranchLength>) -> Self {
        Vertex::Internal {
            index,
            parent: NO_PARENT_SET,
            children,
            branch_length,
        }
    }

    /// Creates a new leaf vertex.
    ///
    /// # Arguments
    /// * `index` - The unique index of this vertex in the tree (arena)
    /// * `branch_length` - Distance to parent node (non-negative)
    /// * `label` -  Label (reference) for this leaf (type depends on tree variant)
    pub fn new_leaf(index: VertexIndex, branch_length: Option<BranchLength>, label: L) -> Self {
        Vertex::Leaf {
            index,
            label,
            parent: NO_PARENT_SET,
            branch_length,
        }
    }

    /// Returns the index of this vertex.
    pub fn index(&self) -> VertexIndex {
        match self {
            Vertex::Root { index, .. } => *index,
            Vertex::Internal { index, .. } => *index,
            Vertex::Leaf { index, .. } => *index,
        }
    }

    /// Returns whether this vertex has a [BranchLength].
    pub fn has_branch_length(&self) -> bool {
        match self {
            Vertex::Root { branch_length, .. } => branch_length.is_some(),
            Vertex::Internal { branch_length, .. } => branch_length.is_some(),
            Vertex::Leaf { branch_length, .. } => branch_length.is_some(),
        }
    }

    /// Returns the branch length if this is a non-root vertex, else `None`.
    pub fn branch_length(&self) -> Option<BranchLength> {
        match self {
            Vertex::Root { .. } => None,
            Vertex::Internal { branch_length, .. } => *branch_length,
            Vertex::Leaf { branch_length, .. } => *branch_length,
        }
    }

    /// Returns label if this is a leaf, else `None`.
    pub fn label(&self) -> Option<&L> {
        match self {
            Vertex::Leaf { label, .. } => Some(label),
            _ => None,
        }
    }

    /// Returns `true` if this vertex is a leaf.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Vertex::Leaf { .. })
    }

    /// Returns `true` if this vertex is an internal vertex.
    pub fn is_internal(&self) -> bool {
        matches!(self, Vertex::Internal { .. })
    }

    /// Returns indices of the children if this vertex has any, else `None`.
    pub fn children(&self) -> Option<(usize, usize)> {
        match self {
            Vertex::Root { children, .. } => Some(*children),
            Vertex::Internal { children, .. } => Some(*children),
            Vertex::Leaf { .. } => None,
        }
    }

    /// Returns `true` if this vertex is a root.
    pub fn is_root(&self) -> bool {
        matches!(self, Vertex::Root { .. })
    }

    /// Sets new parent for non-root vertex.
    ///
    /// # Panics
    /// Panics if called on root.
    pub fn set_parent(&mut self, parent: VertexIndex) {
        match self {
            Vertex::Root { .. } => panic!("Cannot set parent on root vertex"),
            Vertex::Internal { parent: p, .. } => *p = parent,
            Vertex::Leaf { parent: p, .. } => *p = parent,
        }
    }

    /// Returns the index of parent if this a non-root vertex, else `None`.
    ///
    /// Note that parent might not be set yet during construction.
    pub fn parent_index(&self) -> Option<usize> {
        match self {
            Vertex::Internal { parent, .. } | Vertex::Leaf { parent, .. } => {
                if *parent == NO_PARENT_SET {
                    None
                } else {
                    Some(*parent)
                }
            }
            Vertex::Root { .. } => None,
        }
    }

    /// Returns `true` if this vertex has a parent set.
    pub fn has_parent(&self) -> bool {
        match self {
            Vertex::Internal { parent, .. } | Vertex::Leaf { parent, .. } => {
                *parent != NO_PARENT_SET
            }
            Vertex::Root { .. } => false,
        }
    }
}

impl<L: fmt::Display> fmt::Display for Vertex<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Vertex::Root { index, children, branch_length } => {
                write!(
                    f,
                    "Root(idx: {}, children: [{}, {}], len: {:?})",
                    index, children.0, children.1, branch_length
                )
            }
            Vertex::Internal { index, parent, children, branch_length } => {
                write!(
                    f,
                    "Internal(idx: {}, parent: {}, children: [{}, {}], len: {:?})",
                    index, parent, children.0, children.1, branch_length
                )
            }
            Vertex::Leaf { index, label, parent, branch_length } => {
                write!(
                    f,
                    "Leaf(idx: {}, label: {}, parent: {}, len: {:?})",
                    index, label, parent, branch_length
                )
            }
        }
    }
}

// =#========================================================================#=
// BRANCH LENGTH
// =#========================================================================$=
/// Branch length in a phylogenetic tree, enforced non-negative.
///
/// Represents the evolutionary distance between a vertex and its parent.
/// The value is guaranteed to be non-negative and finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BranchLength(f64);

impl BranchLength {
    /// Creates a new branch length.
    ///
    /// # Arguments
    /// * `length` - The branch length value (must be non-negative)
    ///
    /// # Panics
    /// Panics if `length` is negative or not finite.
    pub fn new(length: f64) -> Self {
        assert!(length >= 0.0, "Branch length must be non-negative, got {}", length);
        assert!(length.is_finite(), "Branch length must be finite, got {}", length);
        BranchLength(length)
    }
}

impl Deref for BranchLength {
    type Target = f64;
    fn deref(&self) -> &f64 {
        &self.0
    }
}

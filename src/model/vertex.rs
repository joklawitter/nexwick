//! Vertex module for phylogenetic tree representation.
#![allow(dead_code)]
// use crate::model::tree::Tree;

use crate::model::tree::{LabelIndex, TreeIndex};
use std::ops::Deref;

/// During construction, Internal and Leaf vertex might not have parent set yet.
const NO_PARENT_SET: TreeIndex = usize::MAX;

// =#========================================================================#=
// VERTEX
// =#========================================================================#=
/// Represents a vertex (node) in a phylogenetic tree.
///
/// A vertex can be either:
/// - **Root**: Has two children, no parent and no branch_length
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
pub enum Vertex {
    /// Root vertex of the tree (has no parent, has two children)
    Root {
        /// Index of this vertex in the tree arena
        index: TreeIndex,
        /// Indices of the two child vertices
        children: (TreeIndex, TreeIndex),
    },
    /// Internal vertex (has parent and two children, no label)
    Internal {
        /// Index of this vertex in the tree arena
        index: TreeIndex,
        /// Index of the parent vertex
        parent: TreeIndex,
        /// Indices of the two child vertices
        children: (TreeIndex, TreeIndex),
        /// Distance to parent node (optional, non-negative if present)
        branch_length: Option<BranchLength>,
    },
    /// Leaf vertex (has parent and label, no children)
    Leaf {
        /// Index of this vertex in the tree arena
        index: TreeIndex,
        /// Index into the shared label map
        label_index: LabelIndex,
        /// Index of the parent vertex
        parent: TreeIndex,
        /// Distance to parent node (optional, non-negative if present)
        branch_length: Option<BranchLength>,
    },
}

impl Vertex {
    /// Creates a new root vertex.
    ///
    /// # Arguments
    /// * `index` - The unique index of this vertex in the tree (arena)
    /// * `children` - Tuple of child indices
    pub fn new_root(index: TreeIndex, children: (TreeIndex, TreeIndex)) -> Self {
        Vertex::Root {
            index,
            children,
        }
    }

    /// Creates a new internal (non-leaf, non-root) vertex .
    ///
    /// # Arguments
    /// * `index` - The unique index of this vertex in the tree (arena)
    /// * `children` - Tuple of child indices
    /// * `branch_length` - Distance to parent node (non-negative)
    pub fn new_internal(index: TreeIndex, children: (TreeIndex, TreeIndex), branch_length: Option<BranchLength>) -> Self {
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
    /// * `label_index` - Index into the label map for this leaf's label
    pub fn new_leaf(index: TreeIndex, branch_length: Option<BranchLength>, label_index: LabelIndex) -> Self {
        Vertex::Leaf {
            index,
            label_index,
            parent: NO_PARENT_SET,
            branch_length,
        }
    }

    /// Returns the index of this vertex.
    pub fn index(&self) -> TreeIndex {
        match self {
            Vertex::Root { index, .. } => *index,
            Vertex::Internal { index, .. } => *index,
            Vertex::Leaf { index, .. } => *index,
        }
    }

    /// Returns whether this vertex has a [BranchLength].
    pub fn has_branch_length(&self) -> bool {
        match self {
            Vertex::Root { .. } => true,
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

    /// Returns label index if this is a leaf, else `None`.
    pub fn label_index(&self) -> Option<usize> {
        match self {
            Vertex::Leaf { label_index, .. } => Some(*label_index),
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

    /// Returns the children if this is an internal vertex, else `None`.
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
    pub fn set_parent(&mut self, parent: TreeIndex) {
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


// =#========================================================================#=
// BRANCH LENGTH
// =#========================================================================#=
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

// impl From<f64> for BranchLength {
//     fn from(value: f64) -> Self {
//         BranchLength::new(value)
//     }
// }

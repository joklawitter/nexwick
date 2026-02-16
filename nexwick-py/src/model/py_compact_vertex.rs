//! Python wrapper for [Vertex]<String> - the simple vertex variant where
//! labels are stored directly as strings.

use nexwick::model::Vertex;
use pyo3::prelude::*;

/// A vertex (node) in a phylogenetic tree.
///
/// A vertex can be one of three types:
/// - **Root**: Has two children, no parent (check with `is_root()`)
/// - **Internal**: Has two children and a parent, no label (check with `is_internal()`)
/// - **Leaf**: Has a parent and label, no children (check with `is_leaf()`)
///
/// Use the `is_*` methods to determine the vertex type, then access
/// the appropriate properties.
#[pyclass(name = "Vertex")]
#[derive(Clone)]
pub struct PyVertex {
    inner: Vertex<String>,
}

impl PyVertex {
    /// Creates a new PyVertex wrapping a Vertex<String>.
    pub fn new(vertex: Vertex<String>) -> Self {
        PyVertex { inner: vertex }
    }

    /// Returns a reference to the inner Vertex.
    pub fn inner(&self) -> &Vertex<String> {
        &self.inner
    }
}

#[pymethods]
impl PyVertex {
    /// Returns the index of this vertex in the tree.
    #[getter]
    fn index(&self) -> usize {
        self.inner.index()
    }

    /// Returns `True` if this vertex is the root of the tree.
    fn is_root(&self) -> bool {
        self.inner.is_root()
    }

    /// Returns `True` if this vertex is an internal (non-leaf, non-root) vertex.
    fn is_internal(&self) -> bool {
        self.inner.is_internal()
    }

    /// Returns `True` if this vertex is a leaf.
    fn is_leaf(&self) -> bool {
        self.inner.is_leaf()
    }

    /// Returns the label if this is a leaf vertex, otherwise `None`.
    #[getter]
    fn label(&self) -> Option<String> {
        self.inner.label().cloned()
    }

    /// Returns the branch length (distance to parent) if set, otherwise `None`.
    ///
    /// Note: Root vertices typically don't have a branch length.
    #[getter]
    fn branch_length(&self) -> Option<f64> {
        self.inner.branch_length().map(|bl| *bl)
    }

    /// Returns `True` if this vertex has a branch length set.
    fn has_branch_length(&self) -> bool {
        self.inner.has_branch_length()
    }

    /// Returns the indices of the two children if this vertex has children
    /// (i.e., is a root or internal vertex), otherwise `None`.
    #[getter]
    fn children(&self) -> Option<(usize, usize)> {
        self.inner.children()
    }

    /// Returns the index of the parent vertex if this is not the root,
    /// otherwise `None`.
    #[getter]
    fn parent(&self) -> Option<usize> {
        self.inner.parent()
    }

    /// Returns `True` if this vertex has a parent set.
    fn has_parent(&self) -> bool {
        self.inner.has_parent()
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }

    fn __str__(&self) -> String {
        format!("{}", self.inner)
    }
}

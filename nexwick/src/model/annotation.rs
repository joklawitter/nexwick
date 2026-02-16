//! Vertex annotations for phylogenetic trees.
//!
//! Provides the [Annotations] struct, which can store parsed annotation values
//! for vertices based on their indices. Supported values captured by
//! [AnnotationValue] are `f64`, `i64`, and `String`.

use crate::model::VertexIndex;
use std::collections::HashMap;
use std::string::String;

// =#========================================================================#=
// ANNOTATION
// =#========================================================================$=
/// Vertex annotations for multiple keys
#[derive(Debug, Clone)]
pub struct Annotations {
    annotations: HashMap<String, Vec<Option<AnnotationValue>>>,
    num_vertices: usize,
}

impl Annotations {
    /// Creates a new empty [Annotations] for a tree with `num_vertices` vertices.
    pub fn new(num_vertices: usize) -> Self {
        Annotations {
            num_vertices,
            annotations: HashMap::new(),
        }
    }

    /// Returns all values for a given annotation key, one per vertex.
    ///
    /// # Arguments
    /// * `key` - Annotation name (e.g. "rate", "height")
    ///
    /// # Returns
    /// [None] if the key does not exist, otherwise a [Vec] parallel to the
    /// tree's vertex arena where each entry is [Some] if that vertex has a
    /// value for this key.
    pub fn get_all_for_key(&self, key: &str) -> Option<&Vec<Option<AnnotationValue>>> {
        self.annotations.get(key)
    }

    /// Returns a single annotation value for a vertex.
    ///
    /// # Arguments
    /// * `key` - Annotation name
    /// * `vertex_index` - Index of the vertex
    ///
    /// # Panics
    /// Panics if `vertex_index` is out of bounds.
    pub fn get(&self, key: &str, vertex_index: VertexIndex) -> Option<AnnotationValue> {
        self.annotations
            .get(key)
            .and_then(|a| a[vertex_index].clone())
    }

    /// Adds an annotation value for a vertex.
    ///
    /// # Arguments
    /// * `key` - Annotation name
    /// * `vertex_index` - Index of the vertex
    /// * `value` - The [AnnotationValue] to store
    ///
    /// # Panics
    /// Panics if `vertex_index` is out of bounds.
    pub fn add(&mut self, key: String, vertex_index: VertexIndex, value: AnnotationValue) {
        let column = self
            .annotations
            .entry(key)
            .or_insert_with(|| vec![None; self.num_vertices]);
        column[vertex_index] = Some(value);
    }
}

// =#========================================================================#=
// ANNOTATION VALUE
// =#========================================================================€=
/// Enum to encapsulate a parsed annotation value.
#[derive(Debug, Clone)]
pub enum AnnotationValue {
    /// For floating point values
    Float(f64),
    /// For integer values
    Int(i64),
    /// For strings
    String(String),
}

impl From<f64> for AnnotationValue {
    fn from(v: f64) -> Self {
        AnnotationValue::Float(v)
    }
}

impl From<f32> for AnnotationValue {
    fn from(v: f32) -> Self {
        AnnotationValue::Float(v as f64)
    }
}

impl From<i64> for AnnotationValue {
    fn from(v: i64) -> Self {
        AnnotationValue::Int(v)
    }
}

impl From<i32> for AnnotationValue {
    fn from(v: i32) -> Self {
        AnnotationValue::Int(v as i64)
    }
}

impl From<String> for AnnotationValue {
    fn from(v: String) -> Self {
        AnnotationValue::String(v)
    }
}

impl From<&str> for AnnotationValue {
    fn from(v: &str) -> Self {
        AnnotationValue::String(v.to_string())
    }
}

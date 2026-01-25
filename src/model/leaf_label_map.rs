//! Provides [LeafLabelMap] for [CompactTree](crate::model::CompactTree)s.
//!
//! [LeafLabelMap] implements joined storage and lookup
//! for leaf labels of trees on the same labels/taxa.
//! Uses type alias [LabelIndex] for indices.

use crate::model::label_storage::LabelStorage;
use std::collections::HashMap;
use std::fmt;

/// Index of a leaf label in a [LeafLabelMap].
pub type LabelIndex = usize;

// =#========================================================================#=
// LEAF LABEL MAP
// =#========================================================================#=
/// Maps leaf labels (strings) to indices and vice versa.
///
/// This bidirectional mapping allows multiple trees with the same taxa to
/// share a single label storage, with each leaf referencing labels by
/// [LabelIndex]. Labels are deduplicated automatically: inserting the same
/// label twice returns the same index.
///
/// # Example
/// ```
/// use nexwick::model::leaf_label_map::LeafLabelMap;
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
    /// List of unique labels with its index used as id;
    /// index is this vector must be stored in map for this label.
    labels: Vec<String>,
    /// Map from label to its index in the labels vector;
    /// stored index must be index in vector for this label.
    map: HashMap<String, LabelIndex>,
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
    /// Prefer [get_or_insert](LeafLabelMap::get_or_insert) which handles deduplication.
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
    pub fn get_index(&self, label: &str) -> Option<LabelIndex> {
        self.map.get(label).map(|&index| index)
    }

    /// Retrieves the leaf label for a given index.
    ///
    /// # Arguments
    /// * `index` - The index to look up
    ///
    /// # Returns
    /// `Some(&str)` if the index is valid, `None` otherwise
    pub fn get_label(&self, index: LabelIndex) -> Option<&str> {
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

    /// Returns reference to the labels in this map.
    pub fn labels(&self) -> &Vec<String> {
        &self.labels
    }

    /// Returns reference to the underlying map.
    pub fn map(&self) -> &HashMap<String, usize> {
        &self.map
    }
}

impl LabelStorage for LeafLabelMap {
    type LabelRef = LabelIndex;

    fn with_capacity(num_labels: usize) -> Self {
        Self::new(num_labels)
    }

    fn store_and_ref(&mut self, label: &str) -> LabelIndex {
        self.get_or_insert(label)
    }

    fn check_and_ref(&self, label: &str) -> Option<LabelIndex> {
        self.get_index(label)
    }

    fn index_to_ref(&self, index: usize) -> LabelIndex {
        index
    }

    fn num_labels(&self) -> usize {
        self.num_labels()
    }
}

impl fmt::Display for LeafLabelMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "LeafLabelMap ({}/{} labels):",
            self.labels.len(),
            self.num_leaves
        )?;
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

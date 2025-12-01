//! Leaf label module for phylogenetic tree representation.
//!
//! - `LeafLabelMap`: Joined storage and lookup for leaf labels for trees on same labels.

use crate::model::tree::LabelIndex;
use std::collections::HashMap;
use std::fmt;

// =#========================================================================#=
// LEAF LABEL MAP
// =#========================================================================#=
/// Maps leaf labels (strings) to compact indices for efficient storage.
///
/// This bidirectional mapping allows multiple trees with the same taxa to share
/// a single label storage, with each leaf referencing labels by [LabelIndex].
/// Labels are deduplicated automatically - inserting the same label twice returns
/// the same index.
///
/// # Example
/// ```
/// use nexus_parser::model::leaf_label_map::LeafLabelMap;
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

    /// Returns reference to the labels in this map.
    pub fn labels(&self) -> &Vec<String> {
        &self.labels
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
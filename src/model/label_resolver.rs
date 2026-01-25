//! Provides [LabelResolver] to handle label mappings during parsing.
//!
//! [LabelResolver] wraps a [LabelStorage]
//! and handles translation during parsing. For Nexus files with TRANSLATE
//! commands, it maps keys to actual labels before calling the storage.

use crate::model::LabelStorage;
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display};

// =#========================================================================#=
// LABEL RESOLVER
// =#========================================================================€=
/// Resolves labels in Newick strings during parsing, using a [LabelStorage] backend.
///
/// Different variants handle different scenarios:
/// - [VerbatimLabels](Self::VerbatimLabels) — raw Newick files or NEXUS without TRANSLATE
/// - [NexusLabels](Self::NexusLabels) — NEXUS with arbitrary TRANSLATE keys
/// - [NexusIntegerLabels](Self::NexusIntegerLabels) — NEXUS with integer TRANSLATE keys (optimized)
///
/// The resolver converts string labels from the parser into [`LabelStorage::LabelRef`] values,
/// which can then be stored in tree leaves.
#[derive(Debug)]
pub enum LabelResolver<S: LabelStorage> {
    /// Resolves and stores labels verbatim.
    ///
    /// Use for:
    /// - Raw Newick strings/files (without extra translation map)
    /// - Nexus file without TRANSLATE command
    ///
    /// Each label string is passed directly to the [LabelStorage].
    VerbatimLabels(S),

    /// Resolves labels using Nexus TRANSLATE command mapping.
    ///
    /// Following specification, tries to resolve in order:
    /// 1. Key provided by TRANSLATE map
    ///     (e.g. "terny" -> "White-fronted tern")
    /// 2. Integer as 1-based index of label in TAXA block
    ///     (e.g. 12 -> "White-fronted tern")
    /// 3. Verbatim label match
    ///     "White-fronted tern" -> "White-fronted tern"
    ///
    /// Use for:
    /// - Nexus file with generic TRANSLATE command
    NexusLabels {
        /// Pre-computed mapping: TRANSLATE key -> storage reference
        index_map: HashMap<String, S::LabelRef>,
        /// The label storage backend
        storage: S,
    },

    /// Resolves labels using integer-only TRANSLATE keys.
    ///
    /// Optimized for TRANSLATE blocks with consecutive integer keys
    /// `(1, 2, 3, ...)` (1-indexed), enabling direct array lookup
    /// instead of hash map access.
    ///
    /// Only accepts integer labels; fails on non-integer input.
    NexusIntegerLabels {
        /// Direct mapping: array index → storage reference (0-based internally)
        index_array: Vec<S::LabelRef>,
        /// The label storage backend
        storage: S,
    },
}

impl<S: LabelStorage> LabelResolver<S> {
    /// Creates a [VerbatimLabels](Self::VerbatimLabels) resolver.
    ///
    /// Labels are passed directly to the storage as encountered.
    ///
    /// # Arguments
    /// * `storage` - The [LabelStorage] backend to populate
    pub(crate) fn new_verbatim_labels_resolver(storage: S) -> Self {
        LabelResolver::VerbatimLabels(storage)
    }

    /// Creates a [NexusLabels](Self::NexusLabels) resolver.
    ///
    /// Builds a lookup map from TRANSLATE keys to storage references.
    ///
    /// # Arguments
    /// * `translation` - TRANSLATE block mapping (key → full taxon label)
    /// * `storage` - The [LabelStorage] backend (must already contain all labels)
    ///
    /// # Panics
    /// Panics if any label in `translation` is not found in `storage`.
    pub(crate) fn new_nexus_labels_resolver(
        translation: HashMap<String, String>,
        storage: S,
    ) -> Self {
        // Instead of going from key -> label and then from label -> LabelRef,
        // we create a direct mapping key -> LabelRef
        let mut index_map = HashMap::with_capacity(translation.len());
        for (key, actual_label) in &translation {
            let label_index = storage.check_and_ref(&actual_label).expect(&format!(
                "Label {} provided by translation to resolver\
                 not present in provided label storage.",
                actual_label
            ));
            index_map.insert(key.clone(), label_index);
        }

        LabelResolver::NexusLabels { index_map, storage }
    }

    /// Creates a [NexusIntegerLabels](Self::NexusIntegerLabels) resolver.
    ///
    /// Builds a direct lookup array from integer TRANSLATE keys to storage references.
    ///
    /// # Arguments
    /// * `translation` - TRANSLATE block mapping (integer key as string → full taxon label)
    /// * `storage` - The [LabelStorage] backend (must already contain all labels)
    ///
    /// # Panics
    /// Panics if:
    /// - Any key is not a valid positive integer
    /// - Any key is out of bounds (must be 1..=num_labels)
    /// - Any label is not found in `storage`
    /// - Keys are not consecutive/complete (missing indices)
    pub(crate) fn new_nexus_integer_labels_resolver(
        translation: HashMap<String, String>,
        storage: S,
    ) -> Self {
        let num_labels = storage.num_labels();

        // Validate all keys are valid integers and build index array;
        // Array at position `i` contains the label index for NEXUS index `i`
        // (1-based, so NEXUS "1" is at index_array[0])
        let mut index_array: Vec<Option<S::LabelRef>> = vec![None; num_labels];

        for (key, actual_label) in &translation {
            // Parse key as integer
            let nexus_index = key
                .parse::<usize>()
                .expect(&format!("TRANSLATE key '{}' is not a valid integer", key));

            // Validate bounds (1-based NEXUS indexing)
            if nexus_index == 0 || nexus_index > num_labels {
                panic!(
                    "TRANSLATE index {} out of bounds (1-based indexing), valid range: 1-{})",
                    nexus_index, num_labels
                );
            }

            // Look up the label in the label storage
            let label_ref = storage.check_and_ref(actual_label).expect(&format!(
                "Label '{}' provided by translation to resolver\
                 not present in provided label storage.",
                actual_label
            ));

            // Store in array (converting from 1-based to 0-based indexing)
            index_array[nexus_index - 1] = Some(label_ref);
        }

        // Check all labels have been provided and take them out of Some
        let index_array: Vec<S::LabelRef> = index_array
            .into_iter()
            .enumerate()
            .map(|(i, opt)| opt.expect(&format!("Missing translation for index {}", i + 1)))
            .collect();

        LabelResolver::NexusIntegerLabels {
            index_array,
            storage,
        }
    }

    /// Resolves a parsed label string to its storage reference.
    ///
    /// Resolution behavior depends on the variant:
    /// - [VerbatimLabels](Self::VerbatimLabels): stores label and returns reference
    /// - [NexusLabels](Self::NexusLabels): tries TRANSLATE key, then integer index, then verbatim
    /// - [NexusIntegerLabels](Self::NexusIntegerLabels): parses as integer index only
    ///
    /// # Arguments
    /// * `parsed_label` - The label string extracted from the Newick tree
    ///
    /// # Returns
    /// * `Ok(LabelRef)` - The resolved storage reference
    /// * `Err(LabelResolvingError)` - If the label cannot be resolved
    pub(crate) fn resolve_label(
        &mut self,
        parsed_label: &str,
    ) -> Result<S::LabelRef, LabelResolvingError> {
        match self {
            LabelResolver::VerbatimLabels(storage) => Ok(storage.store_and_ref(parsed_label)),

            LabelResolver::NexusLabels { index_map, storage } => {
                // 1. Try if parsed label is key of translation map
                if let Some(label_ref) = index_map.get(parsed_label) {
                    return Ok(label_ref.clone());
                }

                // 2. Try if parsed label is integer
                if let Ok(nexus_index) = parsed_label.parse::<usize>() {
                    if nexus_index == 0 || nexus_index > storage.num_labels() {
                        return Err(LabelResolvingError(format!(
                            "Nexus label index {nexus_index} out of\
                            bounds (1-based indexing, max {})",
                            storage.num_labels()
                        )));
                    }
                    return Ok(storage.index_to_ref(nexus_index - 1));
                }

                // 3. Try if parsed label is verbatim label
                if let Some(verbatim_try) = storage.check_and_ref(parsed_label) {
                    return Ok(verbatim_try);
                }

                Err(LabelResolvingError(format!(
                    "NexusResolver could not resolve {parsed_label}"
                )))
            }

            LabelResolver::NexusIntegerLabels { index_array, .. } => {
                // Try if parsed label is integer (1-based index)
                if let Ok(nexus_index) = parsed_label.parse::<usize>() {
                    // Validate bounds (1-based NEXUS indexing)
                    if nexus_index == 0 || nexus_index > index_array.len() {
                        return Err(LabelResolvingError(format!(
                            "Index {} out of bounds (1-based indexing, valid range: 1-{})",
                            nexus_index,
                            index_array.len()
                        )));
                    }
                    // Convert 1-based to 0-based and lookup in array
                    return Ok(index_array[nexus_index - 1].clone());
                }

                Err(LabelResolvingError(format!(
                    "NexusIntegerLabels resolver requires integer labels, got '{}'",
                    parsed_label
                )))
            }
        }
    }

    /// Consumes the resolver and returns the underlying storage.
    ///
    /// Use this to retrieve the [LabelStorage] after parsing is complete,
    /// e.g., to access accumulated labels or shared storage across trees
    /// such as [LeafLabelMap](crate::model::LeafLabelMap).
    pub(crate) fn into_label_storage(self) -> S {
        match self {
            LabelResolver::VerbatimLabels(storage) => storage,
            LabelResolver::NexusLabels { storage, .. } => storage,
            LabelResolver::NexusIntegerLabels { storage, .. } => storage,
        }
    }

    /// Returns a reference to the underlying storage.
    pub(crate) fn label_storage(&self) -> &S {
        match self {
            LabelResolver::VerbatimLabels(storage) => &storage,
            LabelResolver::NexusLabels { storage, .. } => &storage,
            LabelResolver::NexusIntegerLabels { storage, .. } => &storage,
        }
    }
}

impl<S: LabelStorage> Display for LabelResolver<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LabelResolver::VerbatimLabels(_) => {
                writeln!(f, "LabelResolver::VerbatimLabels")
            }
            LabelResolver::NexusLabels { index_map, .. } => {
                writeln!(f, "LabelResolver::NexusLabels with internal mapping:")?;
                for (key, value) in index_map {
                    writeln!(f, "  {} -> {}", key, value)?;
                }
                Ok(())
            }
            LabelResolver::NexusIntegerLabels { index_array, .. } => {
                writeln!(f, "LabelResolver::NexusIntegerLabels with array mapping:")?;
                for (i, label_index) in index_array.iter().enumerate() {
                    writeln!(f, "  {} -> {}", i + 1, label_index)?;
                }
                Ok(())
            }
        }
    }
}

// =#========================================================================#=
// LABEL RESOLVING ERROR
// =#========================================================================$=
/// Error returned when [LabelResolver] cannot resolve a label.
#[derive(Debug)]
pub struct LabelResolvingError(String);

impl Display for LabelResolvingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

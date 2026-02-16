//!  Trait for label storage backends used by tree builder and label resolver.

use std::fmt::{Debug, Display};

// =#========================================================================#=
// LABEL STORAGE
// =#========================================================================T=
/// Backend storage for labels during parsing.
///
/// A [LabelStorage] works with a [TreeBuilder](crate::model::TreeBuilder)
/// to handle leaf labels. During parsing, the
/// [LabelResolver](crate::model::LabelResolver) calls storage methods to
/// convert label strings into [LabelRef](Self::LabelRef) values that get
/// passed to the builder.
///
/// The associated type [LabelRef](Self::LabelRef) must match
/// [`TreeBuilder::LabelRef`](crate::model::TreeBuilder::LabelRef).
///
/// # Implementations
/// * [SimpleLabelStorage](crate::model::SimpleLabelStorage):
///   returns owned [String]s
/// * [LeafLabelMap](crate::model::LeafLabelMap):
///   returns indices into shared storage
pub trait LabelStorage: Debug {
    /// The reference type stored in tree leaves.
    type LabelRef: Clone + Display + Debug;

    /// Creates a new storage with capacity for the expected number of labels.
    ///
    /// Called by the parser or tree builder when parsing the Nexus taxa block
    /// or the first Newick string.
    fn with_capacity(num_labels: usize) -> Self;

    /// Stores a label and returns its reference.
    ///
    /// Called during verbatim parsing when labels are added as encountered.
    fn store_and_ref(&mut self, label: &str) -> Self::LabelRef;

    /// Looks up an existing label, returning its reference if found.
    ///
    /// Does not modify storage. Used by NEXUS resolvers to map translation labels.
    fn check_and_ref(&self, label: &str) -> Option<Self::LabelRef>;

    /// Returns the reference for a label by its 0-based index.
    ///
    /// Used for NEXUS integer label resolution (after converting from 1-based).
    fn index_to_ref(&self, index: usize) -> Self::LabelRef;

    /// Returns the number of labels in storage.
    fn num_labels(&self) -> usize;
}

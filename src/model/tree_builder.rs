//! Trait for constructing phylogenetic trees during parsing.
//!
//! The [`TreeBuilder`] trait decouples parsers from concrete tree representations.
//! Parsers call builder methods as they read Newick or Nexus syntax, and the
//! builder assembles whatever tree structure it wants.
//!
//! # Built-in implementations
//! * [`CompactTreeBuilder`] - Builds [`CompactTree`] with labels stored in a shared [`LeafLabelMap`]
//! * [`SimpleTreeBuilder`] - Builds [`SimpleTree`] with labels embedded directly in leaves
//!
//! # Custom implementations
//! You can implement [TreeBuilder] to construct your own tree representation,
//! allowing you to reuse the parsing logic without adopting this library's tree model.
//!
//! # Builder lifecycle
//! A builder can construct multiple trees sequentially:
//!
//! ```text
//! Empty ──→ init_next() ──→ Building ──→ add_*/set_name ──→ finish_tree() ──→ Empty
//!   ↑                                                                           │
//!   └───────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::model::label_resolver::LabelStorage;

// =#========================================================================#=
// TREE BUILDER (trait)
// =#========================================================================T=
/// Abstraction for constructing trees during parsing.
///
/// Parsers are generic over this trait, calling its methods as they encounter
/// leaves, internal nodes, and roots in the input. This allows the same parser
/// to build different tree representations.
///
/// # Implementing this trait
/// Implementors typically maintain internal state for the tree under construction.
/// The parser drives the lifecycle:
///
/// 1. [`init_next`](Self::init_next) -> prepare for a new tree
/// 2. [`add_leaf`](Self::add_leaf), [`add_internal`](Self::add_internal),
///    [`add_root`](Self::add_root) -> build structure
/// 3. [`set_name`](Self::set_name) -> optionally assign a name
/// 4. [`finish_tree`](Self::finish_tree) -> finalize and return the tree
///
/// After [`finish_tree`], the builder returns to an empty state,
/// ready for [`init_next`] again.
///
/// # Implementation strategies
/// There are (at least) two common approaches:
/// * **Separate builder:** A dedicated struct holds construction state and
///   produces the tree on [`finish_tree`]. This keeps the tree type clean and
///   allows the builder to hold temporary data (like a [`LeafLabelMap`] shared
///   across multiple trees).
/// - **Self-building tree:** The tree type implements [`TreeBuilder`]
///   directly, building itself in place. [`finish_tree`] may perform final
///   validation and returns `self`. This avoids an extra type but couples
///   construction logic into the tree.
pub trait TreeBuilder {
    /// The type used to reference labels within the tree.
    ///
    /// For trees with shared label storage, this is typically an index into
    /// a [`LeafLabelMap`]. For self-contained trees, this might be [`String`].
    type LabelRef;

    /// The type used to identify vertices during construction.
    ///
    /// Returned by the `add_*` methods, then passed to subsequent calls to
    /// connect parent-child relationships.
    /// Must be [Copy] + [Clone] since the parser may need to store and
    /// reuse indices. TODO: check if necessary!
    type VertexIdx: Copy + Clone;

    /// The tree type produced by this builder.
    type Tree;

    // TODO
    type Storage: LabelStorage<LabelRef = Self::LabelRef>;

    // TODO
    fn create_storage(capacity: usize) -> Self::Storage;

    // TODO
    fn init_next(&mut self, num_leaves: usize);

    /// TODO
    /// 
    /// # Error Handling
    /// Assuming that most labels are well-formed and can be resolved
    /// when parsing a Nexus file with a TRANSLATE command, which maps
    /// keys in Newick strings to the actual labels, the method does not
    /// return a Result. So implementations must take care to deal with
    /// labels passed on that they cannot resolve. 
    fn add_leaf(&mut self, branch_len: Option<f64>, label: Self::LabelRef) -> Self::VertexIdx;

    /// TODO
    fn add_internal(&mut self, children: (Self::VertexIdx, Self::VertexIdx), branch_len: Option<f64>) -> Self::VertexIdx;

    /// TODO
    fn add_root(&mut self, children: (Self::VertexIdx, Self::VertexIdx), branch_len: Option<f64>) -> Self::VertexIdx;

    /// Sets the name of the currently constructed tree.
    ///
    ///
    fn set_name(&mut self, tree_name: String);

    /// Finalizes the building process and returns the resulting tree.
    ///
    /// Transitions builder from a "construction" state to an "empty" state,
    /// assuming construction of current tree was done and valid.
    ///
    /// # Implementing this method
    /// - **Consumptive implementations:** If the builder is a separate state machine,
    ///   this should ideally leave the builder in an empty state.
    /// - **In-place implementations:** If the implementor is the Tree itself,
    ///   this method may perform final validation and return `self`.
    fn finish_tree(&mut self) -> Option<Self::Tree>;
}
//! Trait for constructing phylogenetic trees during parsing.
//!
//! The [`TreeBuilder`] trait decouples parsers from concrete tree representations.
//! Parsers call builder methods as they read Newick or Nexus syntax, and the
//! builder assembles whatever tree structure it wants.
//!
//! # Label handling
//! A [`TreeBuilder`] works together with a [`LabelStorage`] to handle leaf
//! labels. The key connection is the associated type
//! [`LabelRef`](TreeBuilder::LabelRef):
//!
//! - **[LabelStorage]** gets label strings and returns `LabelRef` values
//! - **[TreeBuilder]** receives those `LabelRef` values in
//!     [`add_leaf`](TreeBuilder::add_leaf)
//!
//! During parsing, a [`LabelResolver`] wraps the storage and handles
//! translation (for Nexus files with TRANSLATE blocks)
//! before calling the storage:
//! - Newick string contains label string/key (e.g., "1" or "Homo_sapiens")
//! - `LabelResolver` translates keys for Nexus, or passes verbatim
//! - `LabelStorage` translates String label into a label (reference) `label_ref`
//! - `TreeBuilder::add_leaf(branch_len, label_ref) `
//!
//! # Built-in implementations
//! * [`CompactTreeBuilder`] - Builds [`CompactTree`] with labels stored in a shared [`LeafLabelMap`]
//! * [`SimpleTreeBuilder`] - Builds [`SimpleTree`] with labels (copies) stored directly in leaves
//!
//! # Custom implementations
//! You can implement [`TreeBuilder`] to construct your own tree representation,
//! allowing you to reuse the parsing logic without adopting this library's tree model.
//! Your implementation must also provide a compatible [`LabelStorage`] via the
//! associated type [`Storage`](TreeBuilder::Storage).
//!
//! # Builder lifecycle
//! A builder can construct multiple trees sequentially:
//!
//! ```text
//! Empty ──→ init_next() ──→ Building ──→ add_*/set_name ──→ finish_tree() ──→ Empty
//!   ↑                                                                           │
//!   └───────────────────────────────────────────────────────────────────────────┘
//! ```
// Imports for doc links
#[allow(unused_imports)]
use crate::model::{
    LabelResolver,
    CompactTreeBuilder,
    CompactTree,
    LeafLabelMap,
    SimpleTreeBuilder,
    SimpleTree,
};

use crate::model::label_storage::LabelStorage;

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
/// After `finish_tree`, the builder returns to an empty state,
/// ready for `init_next` again.
///
/// # Implementation strategies
/// There are (at least) two common approaches:
/// * **Separate builder:** A dedicated struct holds construction state and
///   produces the tree on `finish_tree`. This keeps the tree type clean and
///   allows the builder to hold temporary data (like a [`LeafLabelMap`] shared
///   across multiple trees).
/// - **Self-building tree:** The tree type implements [`TreeBuilder`]
///   directly, building itself in place. `finish_tree` may perform final
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

    /// The [`LabelStorage`] type compatible with this builder.
    ///
    /// The constraint `LabelRef = Self::LabelRef` ensures the storage
    /// produces references that this builder can accept in
    /// [`add_leaf`](Self::add_leaf).
    /// The parser uses [`create_storage`](Self::create_storage) to
    /// instantiate this type before parsing begins.
    type Storage: LabelStorage<LabelRef = Self::LabelRef>;

    /// Creates a new [`LabelStorage`] instance for parsing.
    ///
    /// Called by the parser before parsing begins. The `capacity` hint
    /// is typically the expected number of leaves (from Nexus NTAX).
    fn create_storage(capacity: usize) -> Self::Storage;

    /// Prepares the builder for constructing a new tree.
    ///
    /// Called by the parser before each tree. Implementations should reset
    /// internal state and optionally pre-allocate based on `num_leaves`.
    ///
    /// # Arguments
    /// * `num_leaves` — Expected number of leaves (hint for allocation)
    fn init_next(&mut self, num_leaves: usize);

    /// Adds a leaf vertex to the tree under construction.
    ///
    /// Called when the parser encounters a leaf (taxon) in the Newick string.
    /// Returns a vertex index that the parser will later pass to
    /// [`add_internal`](Self::add_internal) or [`add_root`](Self::add_root)
    /// to establish parent-child relationships.
    ///
    /// # Arguments
    /// * `branch_len` — Branch length to parent, if specified in the Newick
    /// * `label` — Label reference obtained from the [LabelStorage]
    fn add_leaf(&mut self, branch_len: Option<f64>, label: Self::LabelRef) -> Self::VertexIdx;

    /// Adds an internal (non-root) vertex with two children.
    ///
    /// Called when the parser encounters an internal vertex.
    /// The children are vertex indices returned by previous `add_*` calls.
    /// Returns a vertex index that the parser will later pass to
    /// [`add_internal`](Self::add_internal) or [`add_root`](Self::add_root)
    /// to establish parent-child relationships.
    ///
    /// # Arguments
    /// * `children` — Indices of the left and right child vertices
    /// * `branch_len` — Branch length to parent, if specified
    fn add_internal(&mut self, children: (Self::VertexIdx, Self::VertexIdx), branch_len: Option<f64>) -> Self::VertexIdx;

    /// Adds the root vertex, completing the tree structure.
    ///
    /// Called when the parser finished parsing the Newick string
    /// with the root. After this, only [`set_name`](Self::set_name)
    /// and [`finish_tree`](Self::finish_tree) remain.
    ///
    /// # Arguments
    /// * `children` — Indices of the root's two child vertices
    /// * `branch_len` — Root branch length (rare, but allowed in Newick)
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
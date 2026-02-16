//! Structs and logic to parse Nexus files.
//!
//! This module provides the [NexusParserBuilder] and [NexusParser] structs,
//! which offers methods to parse Nexus files with different configurations.

use crate::model::label_storage::LabelStorage;
use crate::model::tree_builder::TreeBuilder;
use crate::model::{CompactTreeBuilder, LabelResolver};
use crate::newick::NewickParser;
use crate::nexus::defs::*;
use crate::nexus::parser::ReadStrategy::Automatic;
use crate::parser::buffered_byte_source::BufferedByteSource;
use crate::parser::byte_parser::{ByteParser, ConsumeMode::*};
use crate::parser::byte_source::ByteSource;
use crate::parser::in_memory_byte_source::InMemoryByteSource;
use crate::parser::parsing_error::ParsingError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// =#========================================================================#=
// PARSING MODE
// =#========================================================================€=
/// Mode of [NexusParser] when parsing the TREES block: eager or lazy.
enum TreeParsingMode<T: TreeBuilder> {
    /// Eagerly parse all trees upfront and store them
    Eager { trees: Vec<T::Tree> },
    /// Lazily parse trees as requested without storing them
    Lazy {
        /// Byte position where the first tree to parse begins (for reset)
        start_byte_pos: usize,
    },
}

// =#========================================================================#=
// BURNIN
// =#========================================================================€=
/// Specifies how many initial trees to skip as burnin.
///
/// Burnin is commonly used in MCMC sampling to discard initial trees
/// before the chain has converged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Burnin {
    /// Skip a fixed number of trees.
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::Burnin;
    /// let burnin = Burnin::Count(1001); // Skip first 1001 trees
    /// ```
    Count(usize),

    /// Skip a percentage of total trees.
    ///
    /// The percentage must be in the range [0.0, 1.0);
    /// behaviour undefined otherwise.
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::Burnin;
    /// let burnin = Burnin::Percentage(0.25); // Skip first 25% of trees
    /// ```
    Percentage(f64),
}

impl Burnin {
    /// Calculates the absolute number of trees to skip given the total tree count.
    ///
    /// # Arguments
    /// * `num_total_trees` - Total number of trees in the file
    ///
    /// # Returns
    /// The number of trees to skip as burnin
    pub(crate) fn get_count(&self, num_total_trees: usize) -> usize {
        match self {
            Burnin::Count(n) => *n,
            Burnin::Percentage(p) => (num_total_trees as f64 * p).floor() as usize,
        }
    }

    /// Returns whether the number of burnin trees is "significant enough" to
    /// not just parse all trees and discard burnin afterwards in eager mode.
    pub(crate) fn significant(&self) -> bool {
        const SIGNIFICANT_BURNIN_THRESHOLD: usize = 100;

        match &self {
            Burnin::Count(n) => *n >= SIGNIFICANT_BURNIN_THRESHOLD,
            Burnin::Percentage(p) => *p >= 0.05,
        }
    }
}

// =#========================================================================#=
// BYTE SOURCE SETTING
// =#========================================================================€=
/// Controls how the file is read during parsing.
///
/// By default, the [NexusParserBuilder] uses [Automatic], which picks a
/// strategy based on file size. Use
/// [with_buffered_source()](NexusParserBuilder::with_buffered_source) or
/// [with_in_memory_source()](NexusParserBuilder::with_in_memory_source)
/// to override this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadStrategy {
    /// Read the file in chunks through a buffered I/O reader.
    Buffered,

    /// Load the entire file into a contiguous byte buffer before parsing.
    InMemory,

    /// Automatically choose between [ReadStrategy::Buffered] and
    /// [ReadStrategy::InMemory] based on file size.
    /// This is the default.
    Automatic,
}

// =#========================================================================#=
// NEXUS PARSER BUILDER
// =#========================================================================$=
/// Builder for configuring and creating a [NexusParser].
///
/// This builder provides an API for configuring how NEXUS files should be
/// parsed before initialization. Once configured, call
/// [`build()`](NexusParserBuilder::build) to create an initialized
/// [NexusParser] ready for tree retrieval.
///
/// Generic over:
/// * `T: TreeBuilder` — the tree builder (default: [CompactTreeBuilder])
///
/// # Configuration Options
/// * **Tree builder**:
///   - [`with_tree_builder()`](Self::with_tree_builder)
///     — Use a custom [TreeBuilder] (or switch to
///     [SimpleTreeBuilder](crate::model::SimpleTreeBuilder))
///
/// * **Parsing mode**: Choose between eager or lazy:
///   - [`eager()`](Self::eager) — Parse and store all trees during build (default)
///   - [`lazy()`](Self::lazy) — Parse trees on-demand
///
/// * **Skip first**: Skip the first tree (some software creates files with
///   e.g. 10001 trees samples)
///   - [`with_skip_first()`](NexusParserBuilder::with_skip_first)
///     — Skip the very first tree
///
/// * **Burnin**: Discard initial trees (commonly used for MCMC samples)
///   - [`with_burnin()`](NexusParserBuilder::with_burnin)
///     — Skip a fixed count or percentage
///
/// * **Annotations**: Parse vertex annotations instead of treating them as comments
///   - [`with_annotations()`](Self::with_annotations)
///     — Enable parsing of `[&key=value,...]` blocks
///
/// # Example
/// ```no_run
/// use nexwick::nexus::{NexusParserBuilder, Burnin};
///
/// // Parse a NEXUS file with 10% burnin, eagerly loading all trees
/// let mut parser = NexusParserBuilder::for_file("passeriformes.trees")?
///     .with_burnin(Burnin::Percentage(0.1))
///     .eager()
///     .build()?;
///
/// // Eager mode: use next_tree_ref() to iterate by reference
/// while let Some(tree) = parser.next_tree_ref() {
///     println!("Tree height: {}", tree.height());
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct NexusParserBuilder<T: TreeBuilder> {
    mode: TreeParsingMode<T>,
    path: PathBuf,
    read_strategy: ReadStrategy,
    burnin: Burnin,
    skip_first: bool,
    parse_annotations: bool,
    tree_builder: T,
}

// ============================================================================
// Building - InMemory-ByteSource specific (pub)
// ============================================================================
impl NexusParserBuilder<CompactTreeBuilder> {
    /// Creates a new builder from a file.
    ///
    /// Entire file is read into memory.
    /// The builder is initialized with default settings:
    /// - Eager parser mode
    /// - First tree not skipped
    /// - No burnin (all trees included)
    ///
    /// # Arguments
    /// * `path` - Path to the file (accepting `&str`, `String`, `Path`, or `PathBuf`)
    ///   with semicolon-separated list of Newick strings
    ///
    /// # Returns
    /// A builder configured with defaults, ready for method chaining
    ///
    /// # Errors
    /// Returns an I/O error if the file cannot be read
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::{NexusParserBuilder, Burnin};
    ///
    /// let parser = NexusParserBuilder::for_file("falconidae.trees")?
    ///     .with_burnin(Burnin::Count(1000))
    ///     .with_skip_first()
    ///     .build()?;
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn for_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        Ok(NexusParserBuilder {
            mode: TreeParsingMode::Eager { trees: Vec::new() },
            path: path.as_ref().to_path_buf(),
            read_strategy: Automatic,
            burnin: Burnin::Count(0),
            skip_first: false,
            parse_annotations: false,
            tree_builder: CompactTreeBuilder::new(),
        })
    }
}

// ============================================================================
// Building - Generic (pub)
// ============================================================================
impl<T: TreeBuilder> NexusParserBuilder<T> {
    /// Configure the parser to use **eager mode** (parse all trees upfront).
    ///
    /// In eager mode, all trees are parsed and stored in memory during the
    /// [`build()`](NexusParserBuilder::build) call. This allows fast iteration
    /// and multiple passes through the trees, but uses more memory for large
    /// datasets.
    ///
    /// This is the **default mode**.
    ///
    /// # Returns
    /// The builder with eager mode configured
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::NexusParserBuilder;
    ///
    /// let parser = NexusParserBuilder::for_file("hirundinidae.trees")?
    ///     .eager()  // Parse all trees during build()
    ///     .build()?;
    /// let (trees, label_storage) = parser.into_results().unwrap();
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn eager(mut self) -> Self {
        self.mode = TreeParsingMode::Eager { trees: Vec::new() };
        self
    }

    /// Configure the parser to use **lazy mode** (parse trees on-demand).
    ///
    /// In lazy mode, trees are parsed one at a time as you call
    /// [next_tree()](NexusParser::next_tree). This uses less memory but
    /// requires reparsing for multiple iterations.
    ///
    /// # Returns
    /// The builder with lazy mode configured
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::NexusParserBuilder;
    ///
    /// let mut parser = NexusParserBuilder::for_file("rhipiduridae.trees")?
    ///     .lazy()  // Parse trees on-demand
    ///     .build()?;
    ///
    /// // Single pass through trees
    /// while let Some(tree) = parser.next_tree()? {
    ///     // Process tree
    /// }
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn lazy(mut self) -> Self {
        self.mode = TreeParsingMode::Lazy { start_byte_pos: 0 };
        self
    }

    /// Configure burnin, i.e., discard/skip initial trees.
    ///
    /// Burnin skips trees from the beginning of the file, either as a fixed
    /// count or as a percentage of total trees. This is commonly used in
    /// Bayesian phylogenetics to discard samples before the MCMC chain has
    /// converged.
    ///
    /// If both burnin and [with_skip_first()](NexusParserBuilder::with_skip_first)
    /// are configured, the first tree is skipped, then burnin is applied to
    /// the remaining trees.
    ///
    /// # Returns
    /// The builder with burnin configured
    ///
    /// # Arguments
    /// * `burnin` - The burnin specification ([Burnin::Count] or [Burnin::Percentage])
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::{NexusParserBuilder, Burnin};
    ///
    /// // Skip first 1000 trees
    /// let parser = NexusParserBuilder::for_file("sylviidae.trees")?
    ///     .with_burnin(Burnin::Count(1000))
    ///     .build()?;
    ///
    /// // Skip first 1% of trees
    /// let parser = NexusParserBuilder::for_file("sylviidae.trees")?
    ///     .with_burnin(Burnin::Percentage(0.01))
    ///     .build()?;
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_burnin(mut self, burnin: Burnin) -> Self {
        self.burnin = burnin;
        self
    }

    /// Configure the parser to skip the first tree.
    ///
    /// This is useful when software provides samples with, say, 10001 trees,
    /// where the first tree might be the start tree of an MCMC run. For easier
    /// reporting and clearer statistics, one might want to skip that one tree.
    ///
    /// If both [with_skip_first()](NexusParserBuilder::with_skip_first) and
    /// [with_burnin()](NexusParserBuilder::with_burnin) are configured, the
    /// first tree is skipped, then burnin is applied to the remaining trees.
    ///
    /// # Returns
    /// The builder with skip-first configured
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::{NexusParserBuilder, Burnin};
    ///
    /// // Skip first tree (often a consensus tree)
    /// let parser = NexusParserBuilder::for_file("paradisaeidae.trees")?
    ///     .with_skip_first()
    ///     .build()?;
    ///
    /// // Skip first tree AND apply 3% burnin to the rest
    /// let parser = NexusParserBuilder::for_file("paradisaeidae.trees")?
    ///     .with_skip_first()
    ///     .with_burnin(Burnin::Percentage(0.03))
    ///     .build()?;
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_skip_first(mut self) -> Self {
        self.skip_first = true;
        self
    }

    /// Configure the parser to parse vertex annotations
    /// (e.g. `[&rate=0.5,pop_size=1.2]`) instead of treating them as comments.
    pub fn with_annotations(mut self) -> Self {
        self.parse_annotations = true;
        self
    }

    /// Configure the parser to read the file using a **buffered reader**.
    ///
    /// The file is read in chunks through a buffered I/O reader, keeping
    /// memory usage low regardless of file size. This is suitable for
    /// large files where loading everything into memory is undesirable.
    ///
    /// See also [with_in_memory_source()](Self::with_in_memory_source).
    ///
    /// # Returns
    /// The builder with buffered source configured
    pub fn with_buffered_source(mut self) -> Self {
        self.read_strategy = ReadStrategy::Buffered;
        self
    }

    /// Configure the parser to read the **entire file into memory** upfront.
    ///
    /// The file contents are loaded into a contiguous byte buffer before
    /// parsing begins. This avoids repeated I/O during parsing, which can
    /// be faster for small and moderately sized files.
    ///
    /// See also [with_buffered_source()](Self::with_buffered_source).
    ///
    /// # Returns
    /// The builder with in-memory source configured
    pub fn with_in_memory_source(mut self) -> Self {
        self.read_strategy = ReadStrategy::InMemory;
        self
    }

    /// Configure the parser to use custom [TreeBuilder].
    ///
    /// Instead of using the default [CompactTreeBuilder], another
    /// implementation of [TreeBuilder] can be provided. This is then
    /// used when parsing Newick strings to let it build trees.
    /// Furthermore, it must provide a [LabelStorage] that can be used by
    /// the [LabelResolver].
    ///
    /// # Returns
    /// The builder with custom [TreeBuilder] set
    ///
    /// # Example
    /// ```ignore
    /// let parser = NexusParserBuilder::for_file(path)?
    ///     .with_tree_builder(YourTreeBuilder::new())
    ///     .build()?;
    /// ```
    pub fn with_tree_builder<T2: TreeBuilder>(self, tree_builder: T2) -> NexusParserBuilder<T2> {
        NexusParserBuilder {
            mode: match self.mode {
                TreeParsingMode::Eager { .. } => TreeParsingMode::Eager { trees: Vec::new() },
                TreeParsingMode::Lazy { start_byte_pos } => {
                    TreeParsingMode::Lazy { start_byte_pos }
                }
            },
            path: self.path,
            read_strategy: self.read_strategy,
            burnin: self.burnin,
            skip_first: self.skip_first,
            parse_annotations: self.parse_annotations,
            tree_builder,
        }
    }

    /// Builds and initializes the [NexusParser] with the configured settings.
    ///
    /// This method:
    /// 1. Creates the parser with the configured options
    /// 2. Parses the NEXUS header and TAXA block
    /// 3. Parses the TRANSLATE command (if present) in the TREES block
    /// 4. Counts total trees and if burnin set, applies both burnin
    ///    and skip-first setting
    /// 5. In eager mode: parses and stores all remaining trees
    /// 6. In lazy mode: positions the parser at the first tree to return
    ///
    /// After this returns successfully, the parser is ready for tree retrieval
    /// via [`next_tree()`](NexusParser::next_tree) (for lazy/eager mode)
    /// or [`into_results()`](NexusParser::into_results) (for eager mode).
    ///
    /// # Returns
    /// An initialized [NexusParser] ready for tree retrieval
    ///
    /// # Errors
    /// Returns a [ParsingError] if:
    /// - The file is not a valid NEXUS file
    /// - Required blocks (TAXA, TREES) are missing
    /// - The NEXUS format is malformed
    /// - Tree parser fails (in eager mode) for some reason
    ///
    /// # Example
    /// ```no_run
    /// use nexwick::nexus::{NexusParserBuilder, Burnin};
    ///
    /// let parser = NexusParserBuilder::for_file("dinornithidae.trees")?
    ///     .with_burnin(Burnin::Percentage(0.1))
    ///     .eager()    // (Not necessary, because default)
    ///     .build()?;  // Parses NEXUS structure and all trees
    ///
    /// // Parser is now ready
    /// println!("Found {} trees", parser.num_trees());
    ///
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn build(self) -> Result<NexusParser<T>, ParsingError> {
        /// File size threshold (in bytes) for automatic read strategy.
        /// Files smaller than this are read into memory; larger files use buffered I/O.
        const AUTO_IN_MEMORY_THRESHOLD: u64 = 100 * 1024 * 1024; // 100 MB

        let use_buffered = match self.read_strategy {
            ReadStrategy::Buffered => true,
            ReadStrategy::InMemory => false,
            ReadStrategy::Automatic => {
                let file_size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
                file_size >= AUTO_IN_MEMORY_THRESHOLD
            }
        };

        let mut newick_parser = NewickParser::new(self.tree_builder);
        newick_parser.set_parse_annotations(self.parse_annotations);

        if use_buffered {
            let byte_parser = ByteParser::from_file_buffered(&self.path)?;
            let mut inner = NexusParserInner {
                mode: self.mode,
                newick_parser,
                byte_parser,
                num_leaves: 0,
                num_total_trees: 0,
                num_trees: 0,
                start_tree_pos: 0,
                tree_pos: 0,
                burnin: self.burnin,
                skip_first: self.skip_first,
            };
            inner.init()?;
            Ok(NexusParser::Buffered(inner))
        } else {
            let byte_parser = ByteParser::from_file_in_memory(&self.path)?;
            let mut inner = NexusParserInner {
                mode: self.mode,
                newick_parser,
                byte_parser,
                num_leaves: 0,
                num_total_trees: 0,
                num_trees: 0,
                start_tree_pos: 0,
                tree_pos: 0,
                burnin: self.burnin,
                skip_first: self.skip_first,
            };
            inner.init()?;
            Ok(NexusParser::InMemory(inner))
        }
    }
}

// =#========================================================================#=
// NEXUS PARSER
// =#========================================================================$=
/// Parser for NEXUS phylogenetic tree files.
///
/// Created via [NexusParserBuilder]. Provides access to trees parsed from
/// NEXUS format files (BEAST2, MrBayes, RevBayes, etc.).
///
/// # Construction
/// Use [NexusParserBuilder] to configure and create a parser:
///
/// ```no_run
/// use nexwick::nexus::{NexusParserBuilder, Burnin};
///
/// let mut parser = NexusParserBuilder::for_file("numididae.trees")?
///     .with_skip_first()
///     .with_burnin(Burnin::Percentage(0.25))
///     .eager()
///     .build()?;
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Usage
/// After initialization via the builder, use the parser to:
/// - Iterate through trees with [`next_tree()`](Self::next_tree) (lazy mode)
///   or [`next_tree_ref()`](Self::next_tree_ref) (eager mode)
/// - Query metadata like [`num_trees()`](Self::num_trees),
///   [`num_leaves()`](Self::num_leaves)
/// - Access the taxon label storage with [`label_storage()`](Self::label_storage())
/// - Extract all results with [`into_results()`](Self::into_results)
///
/// # Tree Retrieval
///
/// **Eager mode** (default, trees pre-parsed and stored):
/// * [next_tree_ref()](Self::next_tree_ref) — Iterate by reference (no cloning)
/// * [into_results()](Self::into_results) — Consume and get all trees
///
/// ```no_run
/// use nexwick::nexus::{NexusParserBuilder, Burnin};
///
/// let mut parser = NexusParserBuilder::for_file("momotidae.trees")?
///     .eager()    // not necessary, as default
///     .build()?;
/// while let Some(tree) = parser.next_tree_ref() {
///     println!("Height: {}", tree.height());
/// }
///
/// // Extract everything at once
/// let (trees, labels) = parser.into_results()?;
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// **Lazy mode** (trees parsed on-demand):
/// - [`next_tree()`](Self::next_tree) — parse and return next tree
/// - Can use [`reset()`](Self::reset) to iterate multiple times but retrieving
///   trees then requires reparsing
///
/// ```no_run
/// use nexwick::nexus::{NexusParserBuilder, Burnin};
///
/// let mut parser = NexusParserBuilder::for_file("jacanidae.trees")?
///     .lazy()
///     .build()?;
/// while let Some(tree) = parser.next_tree()? {
///     println!("Height: {}", tree.height());
/// }
///
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[allow(private_interfaces)]
pub enum NexusParser<T: TreeBuilder> {
    /// NexusParser with buffered file read
    Buffered(NexusParserInner<BufferedByteSource, T>),
    /// Nexus Parser with in-memory file read
    InMemory(NexusParserInner<InMemoryByteSource, T>),
}

/// Helper macro to delegate a method call to the inner parser variant.
macro_rules! delegate {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self {
            NexusParser::Buffered(inner) => inner.$method($($arg),*),
            NexusParser::InMemory(inner) => inner.$method($($arg),*),
        }
    };
}

impl<T: TreeBuilder> NexusParser<T> {
    /// Reset to first tree (respecting skip-first and burnin setting).
    pub fn reset(&mut self) {
        delegate!(self, reset)
    }

    /// Consumes this [NexusParser] and returns the built [LabelStorage].
    pub fn into_label_storage(self) -> T::Storage {
        delegate!(self, into_label_storage)
    }

    /// Consumes this [NexusParser] and returns the resulting trees
    /// and [LabelStorage].
    pub fn into_results(self) -> Result<(Vec<T::Tree>, T::Storage), ParsingError> {
        delegate!(self, into_results)
    }

    /// Get the number of leaves/taxa based on TAXA block.
    pub fn num_leaves(&self) -> usize {
        delegate!(self, num_leaves)
    }

    /// Get ref to [LabelStorage] of all taxa based on TAXA block.
    pub fn label_storage(&self) -> &T::Storage {
        delegate!(self, label_storage)
    }

    /// Get the number of trees (without burnin trees).
    pub fn num_trees(&self) -> usize {
        delegate!(self, num_trees)
    }

    /// Get the total number of trees including skipped+burnin.
    pub fn num_total_trees(&self) -> usize {
        delegate!(self, num_total_trees)
    }

    /// Returns a reference to the next tree.
    ///
    /// Intended for **eager mode** only. In lazy mode, trees aren't stored,
    /// so this always returns `None`.
    pub fn next_tree_ref(&mut self) -> Option<&T::Tree> {
        delegate!(self, next_tree_ref)
    }

    /// Parses and returns the next tree.
    ///
    /// Intended for **lazy mode** only. In eager mode returns `Ok(None)` as
    /// trees are already parsed — use [next_tree_ref()](Self::next_tree_ref)
    /// or [into_results()](Self::into_results) instead.
    pub fn next_tree(&mut self) -> Result<Option<T::Tree>, ParsingError> {
        delegate!(self, next_tree)
    }
}

// =#========================================================================#=
// NEXUS PARSER INNER
// =#========================================================================$=
/// Inner of [NexusParser] for type erasure pattern of generic byte source.
struct NexusParserInner<B: ByteSource, T: TreeBuilder> {
    /// Mode to parse trees
    mode: TreeParsingMode<T>,
    /// Continuously used to parse Newick strings, including resolving labels
    newick_parser: NewickParser<T>,
    /// Accessor to the underlying bytes/file being parsed
    byte_parser: ByteParser<B>,

    /// Whether to skip the first tree
    skip_first: bool,
    /// Amount of burnin to discard/skip
    burnin: Burnin,

    /// Number of leaves/taxa in all TAXA block and all trees (must be consistent)
    num_leaves: usize,
    /// The total number of `TREE` commands in the Nexus file
    num_total_trees: usize,
    /// The number of `TREE` commands considers afters skipped/discarded ones
    /// - Invariant: `num_trees <= num_total_trees`
    num_trees: usize,
    /// The first `TREE` command to consider (0-indexed)
    /// - Invariant: `num_trees + start_tree_pos = num_total_trees`
    start_tree_pos: usize,
    /// Position of currently next `TREE` command to consider for `next_tree()`
    /// - Invariant: `start_tree_pos <= tree_pos < num_total_trees`
    tree_pos: usize,
}

// ============================================================================
// Initialization & State (private)
// ============================================================================
impl<B: ByteSource, T: TreeBuilder> NexusParserInner<B, T> {
    /// Initializes this [NexusParser] to be ready to retrieve trees.
    ///
    /// Parses the header and TAXA block of the Nexus file, counts the number
    /// of trees, applies burnin and moves to first tree to being parsed.
    /// If configured to be in eager mode, also parses all trees.
    ///
    /// # Returns
    /// * `Ok(())` - If initializing and parsing was successful
    /// * [ParsingError] - If something went wrong during parsing
    fn init(&mut self) -> Result<(), ParsingError> {
        // > Header
        self.parse_nexus_header()?;

        // > TAXA block
        self.skip_until_block(NexusBlock::Taxa)?;
        let label_storage = self.parse_taxa_block()?;

        // > TREES block
        // Skip until TREES block and ...
        self.skip_until_block(NexusBlock::Trees)?;
        // ... handle TRANSLATE command
        let map = self.parse_tree_block_translate()?;

        // ... and based on whether it exists, pick the appropriate label resolver
        let resolver = self.choose_resolver(label_storage, map)?;
        self.newick_parser
            .set_num_leaves(self.num_leaves)
            .set_resolver(resolver);

        // Then move to the first tree
        self.byte_parser.skip_comment_and_whitespace()?;

        // Decide which scenario to use based on mode and burnin significance
        // Scenario 1: Lazy mode - always count first
        // Scenario 2: Eager + "significant" burnin - count first, then skip and parse only what we need
        // -> Scenario 1 & 2 do a two passes over the trees block
        // Scenario 3: Eager + insignificant burnin - parse all, then discard
        // -> one pass over the trees block
        let is_eager = matches!(self.mode, TreeParsingMode::Eager { .. });
        let use_two_pass = !is_eager || self.burnin.significant();

        if use_two_pass {
            // Scenarios 1 & 2: Count trees, configure counts, skip unwanted trees
            let total_trees = self.count_trees()?;
            self.configure_tree_counts(total_trees);

            // Skip past the trees we don't want
            for _ in 0..self.start_tree_pos {
                self.skip_tree()?;
            }

            // For eager mode, parse the trees we want to keep
            if is_eager {
                let mut trees = Vec::with_capacity(self.num_trees);
                self.parse_tree_block_trees(&mut trees)?;
                self.mode = TreeParsingMode::Eager { trees };
            } else {
                // For lazy mode, capture byte position for reset capability
                let start_byte_pos = self.byte_parser.position();
                self.mode = TreeParsingMode::Lazy { start_byte_pos };
            }
        } else {
            // Scenario 3: Eager mode with insignificant burnin (one-pass)
            // Parse all trees, then filter out unwanted ones
            let mut all_trees = Vec::new();
            self.parse_tree_block_trees(&mut all_trees)?;
            self.configure_tree_counts(all_trees.len());

            // Keep only the trees after skip_first and burnin
            let trees = if self.start_tree_pos == 0 {
                // No skipping needed - use the vector directly
                all_trees
            } else {
                // Skip unwanted trees
                let remaining_trees: Vec<_> =
                    all_trees.into_iter().skip(self.start_tree_pos).collect();
                remaining_trees
            };

            self.mode = TreeParsingMode::Eager { trees };
        }

        Ok(())
    }

    /// Helper method to configure tree count fields based on total tree count.
    ///
    /// Sets `num_total_trees`, `num_trees`, `start_tree_pos`, and `tree_pos`
    /// based on `skip_first` and burnin configuration.
    ///
    /// # Arguments
    /// * `num_total_trees` - The total number of trees in the TREES block
    fn configure_tree_counts(&mut self, num_total_trees: usize) {
        self.num_total_trees = num_total_trees;

        // Calculate how many trees to skip from the beginning
        // + skip 1 if configured
        let mut skip_count = 0;
        if self.skip_first && num_total_trees > 0 {
            skip_count = 1;
        }
        // + skip burnin many of the rest
        skip_count += self.burnin.get_count(num_total_trees - skip_count);

        // The actual number of trees that should be parsed (considered):
        self.num_trees = num_total_trees.saturating_sub(skip_count);

        // The position where actual trees start (after skipped and burnin trees)
        self.start_tree_pos = skip_count;
        self.tree_pos = skip_count;
    }

    /// Helper method to pick and configure the right [LabelResolver]
    /// at initialization.
    fn choose_resolver(
        &mut self,
        label_storage: T::Storage,
        map: Option<HashMap<String, String>>,
    ) -> Result<LabelResolver<T::Storage>, ParsingError> {
        Ok(match map {
            None => LabelResolver::new_verbatim_labels_resolver(label_storage),
            Some(map) => {
                // Assert that labels match those provided in TAXA block
                let labels_consistent = map.len() == label_storage.num_labels()
                    && map
                        .values()
                        .all(|label| label_storage.check_and_ref(label).is_some());
                if !labels_consistent {
                    return Err(ParsingError::invalid_translate_command(
                        &mut self.byte_parser,
                    ));
                }

                // Check if all keys are integers to use the more efficient NexusIntegerLabels resolver
                let all_keys_are_integers = map.keys().all(|key| key.parse::<usize>().is_ok());

                if all_keys_are_integers {
                    LabelResolver::new_nexus_integer_labels_resolver(map, label_storage)
                } else {
                    LabelResolver::new_nexus_labels_resolver(map, label_storage)
                }
            }
        })
    }

    /// Reset to first tree (respecting skip-first and burnin setting)
    pub fn reset(&mut self) {
        self.tree_pos = self.start_tree_pos;

        // In lazy mode, also reset the byte parser position
        if let TreeParsingMode::Lazy { start_byte_pos } = self.mode {
            self.byte_parser.set_position(start_byte_pos);
        }
    }
}

// ============================================================================
// De/Construction (pub)
// ============================================================================
impl<B: ByteSource, T: TreeBuilder> NexusParserInner<B, T> {
    /// Consumes this [NexusParser] and returns the build [LabelStorage].
    ///
    /// This extracts the final underlying label storage,
    /// such as the label-to-index mapping used by
    /// [LeafLabelMap](crate::model::LeafLabelMap),
    /// regardless of eager/lazy configuration.
    ///
    /// # Returns
    /// The [LabelStorage] based on the parsed Nexus file.
    pub fn into_label_storage(self) -> T::Storage {
        self.newick_parser.into_label_storage()
    }

    /// Consumes this [NexusParser] and returns the resulting `T::Tree`s
    /// and [LabelStorage].
    ///
    /// This parses all trees and extracts the final underlying label-to-index mapping,
    /// regardless of the eager/lazy configuration.
    ///
    /// # Returns
    /// A vector of the `T::Tree`s and the corresponding [LabelStorage]
    /// from the parsed Nexus file.
    pub fn into_results(mut self) -> Result<(Vec<T::Tree>, T::Storage), ParsingError> {
        match self.mode {
            TreeParsingMode::Eager { trees } => {
                Ok((trees, self.newick_parser.into_label_storage()))
            }
            TreeParsingMode::Lazy { .. } => {
                let mut all_trees = Vec::new();
                self.reset();
                while let Some(tree) = self.next_tree()? {
                    all_trees.push(tree);
                }
                Ok((all_trees, self.newick_parser.into_label_storage()))
            }
        }
    }
}

// ============================================================================
// Getters / Accessors, etc. (pub)
// ============================================================================
impl<B: ByteSource, T: TreeBuilder> NexusParserInner<B, T> {
    /// Get the number of leaves/taxa based on TAXA block
    pub fn num_leaves(&self) -> usize {
        self.num_leaves
    }

    /// Get ref to [LabelStorage] of all taxa based on TAXA block
    pub fn label_storage(&self) -> &T::Storage {
        self.newick_parser.label_storage()
    }

    /// Get the number of trees (without burnin trees)
    pub fn num_trees(&self) -> usize {
        self.num_trees
    }

    /// Get the total number of trees including skipped+burnin
    pub fn num_total_trees(&self) -> usize {
        self.num_total_trees
    }

    /// Returns a reference to the next tree.
    ///
    /// Intended for **eager mode** only. In lazy mode, trees aren't stored,
    /// so this always returns `None`.
    ///
    /// # Returns
    /// * `Some(&Tree)` - Reference to the next tree (eager mode)
    /// * `None` - No more trees, or in lazy mode
    pub fn next_tree_ref(&mut self) -> Option<&T::Tree> {
        match &self.mode {
            TreeParsingMode::Eager { trees } => {
                if self.tree_pos < self.start_tree_pos + self.num_trees {
                    let tree = &trees[self.tree_pos - self.start_tree_pos];
                    self.tree_pos += 1;
                    Some(tree)
                } else {
                    None
                }
            }
            TreeParsingMode::Lazy { .. } => None,
        }
    }

    /// Parses and returns the next tree.
    ///
    /// Intended for **lazy mode** only. In eager mode return `Ok(None)` as
    /// trees are already parsed -> use [`next_tree_ref()`](Self::next_tree_ref)
    /// or [`into_results()`](Self::into_results) instead.
    ///
    /// # Returns
    /// * `Ok(Some(Tree))` - The next tree (owned, lazy mode only)
    /// * `Ok(None)` - No more trees, or in eager mode
    /// * `Err(ParsingError)` - Parsing failed (lazy mode only)
    pub fn next_tree(&mut self) -> Result<Option<T::Tree>, ParsingError> {
        match &self.mode {
            TreeParsingMode::Eager { trees: _ } => {
                // Trees already parsed — use next_tree_ref() instead
                Ok(None)
            }
            TreeParsingMode::Lazy { .. } => {
                // Check if we've reached the end
                if self.tree_pos >= self.start_tree_pos + self.num_trees {
                    return Ok(None);
                }

                // Parse next tree on demand
                let tree = self.parse_single_tree()?;
                if tree.is_none() {
                    return Err(ParsingError::unexpected_eof(&mut self.byte_parser));
                }
                self.tree_pos += 1;
                Ok(tree)
            }
        }
    }
}

// ============================================================================
// Parsing helpers (private)
// ============================================================================
impl<B: ByteSource, T: TreeBuilder> NexusParserInner<B, T> {
    /// Parses header `#NEXUS` at start of file;
    /// returns [`ParsingError::MissingNexusHeader`] if header missing.
    fn parse_nexus_header(&mut self) -> Result<(), ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        if !self.byte_parser.consume_if_sequence(NEXUS_HEADER) {
            return Err(ParsingError::missing_nexus_header(&mut self.byte_parser));
        }

        Ok(())
    }

    /// Skips Nexus blocks until we encounter the target block type, whose
    /// header is consumed;
    /// returns `[ParsingError::UnexpectedEOF]` if block not found.
    fn skip_until_block(&mut self, target: NexusBlock) -> Result<(), ParsingError> {
        loop {
            if self.byte_parser.is_eof() {
                return Err(ParsingError::unexpected_eof(&mut self.byte_parser));
            }

            let block_type = self.detect_next_block()?;

            if block_type == target {
                return Ok(());
            }

            self.skip_to_block_end()?;
        }
    }

    /// Detects the next Nexus block, which must start with header
    /// `BEGIN <BlockType>;` (case-insensitive), consumes its header,
    /// and returns its BlockType, or a [ParsingError] if something went wrong.
    fn detect_next_block(&mut self) -> Result<NexusBlock, ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        if !self.byte_parser.consume_if_sequence(BLOCK_BEGIN) {
            return Err(ParsingError::invalid_formatting(&mut self.byte_parser));
        }
        self.byte_parser.skip_comment_and_whitespace()?;

        let block_name = self.byte_parser.parse_unquoted_label(b";")?;

        self.byte_parser.next_byte(); // consume the ';' now (already know that this is next byte)

        Ok(NexusBlock::from_name(&block_name))
    }

    /// Skips block, e.g. continuing until encountering and consuming `END;`.
    fn skip_to_block_end(&mut self) -> Result<(), ParsingError> {
        if !self
            .byte_parser
            .consume_until_sequence(BLOCK_END, Inclusive)
        {
            return Err(ParsingError::unexpected_eof(&mut self.byte_parser));
        }

        Ok(())
    }

    /// Parses TAXA block extracting number of taxa from `ntax` command
    /// and taxon list from `TAXLABEL` command, ignoring any other command and comments.
    ///
    /// # Assumptions
    /// * First command must be `DIMENSIONS NTAX=<value>;` (case-insensitive)
    ///   - `<value>` must be integer and followed by a semicolon
    ///   - No comment within command allowed
    /// * Followed by list of labels command `TAXLABEL [label1 label2 ...];`,
    ///   with the following details:
    ///   - Space separated list of labels
    ///   - Terminated by semicolon
    ///   - Comments allowed
    /// * Comments allowed outside the two commands
    ///
    /// # Errors
    /// Returns [ParsingError::UnexpectedEOF] if block, command, or comment
    /// not properly closed, and [ParsingErrror::InvalidTaxaBlock] if commands
    /// not encountered in expected order (as specified above).
    fn parse_taxa_block(&mut self) -> Result<T::Storage, ParsingError> {
        // 1. Parse number of taxa command "DIMENSIONS NTAX=n;"
        self.parse_taxa_block_ntax()?;

        // 2. Parse list of taxa labels in `TAXLABEL` command, including consuming closing ";"
        let label_storage = self.parse_taxa_block_labels()?;

        // 3. Move to end of block
        self.skip_to_block_end()?;
        // Would expect only "END;" besides whitespace and comments, but not enforced here

        Ok(label_storage)
    }

    /// Helps parsings TAXA block, responsible for parsing the `ntax` command
    /// and returning the result, i.e. the number of taxa.
    fn parse_taxa_block_ntax(&mut self) -> Result<(), ParsingError> {
        // This could be a replaced with a general parser structure for such a type of command,
        // but for our nexus files we only have this one.
        // a) Parse "DIMENSIONS NTAX="
        self.byte_parser.skip_comment_and_whitespace()?;
        if !self.byte_parser.consume_if_sequence(DIMENSIONS) {
            return Err(ParsingError::invalid_taxa_block(
                &mut self.byte_parser,
                String::from("Expected 'DIMENSIONS' in TAXA block."),
            ));
        }

        self.byte_parser.skip_whitespace();
        if !self.byte_parser.consume_if_sequence(NTAX) {
            return Err(ParsingError::invalid_taxa_block(
                &mut self.byte_parser,
                String::from("Expected 'NTAX' in TAXA block."),
            ));
        }

        self.byte_parser.skip_whitespace();
        if !self.byte_parser.consume_if(b'=') {
            return Err(ParsingError::invalid_taxa_block(
                &mut self.byte_parser,
                String::from("Expected '=' in TAXA block."),
            ));
        }

        // b) Read the number `n` and consume ";"
        self.byte_parser.skip_whitespace();
        let ntax_str = self.byte_parser.parse_unquoted_label(b";")?;
        let ntax: usize = ntax_str.parse().map_err(|_| {
            ParsingError::invalid_taxa_block(
                &mut self.byte_parser,
                format!("Cannot parse `ntax` value: {}", ntax_str),
            )
        })?;
        self.byte_parser.next_byte(); // consume the semicolon

        self.num_leaves = ntax;
        Ok(())
    }

    /// Helps parsing TAXA block, responsible for parsing the `TAXLABEL`
    /// command and returning the parsed taxa as [LabelStorage].
    fn parse_taxa_block_labels(&mut self) -> Result<T::Storage, ParsingError> {
        // a) Parse "TAXLABELS"
        self.byte_parser.skip_comment_and_whitespace()?;
        if !self.byte_parser.consume_if_sequence(TAXLABELS) {
            return Err(ParsingError::invalid_taxa_block(
                &mut self.byte_parser,
                String::from("Expected 'TAXLABELS' in TAXA block."),
            ));
        }

        // b) Read labels until semicolon
        let mut label_storage = T::create_storage(self.num_leaves);
        let mut count = 0;
        loop {
            self.byte_parser.skip_comment_and_whitespace()?;

            // Stop once encountering semicolon (end of labels command)
            if self.byte_parser.peek() == Some(b';') {
                self.byte_parser.next_byte();
                break;
            }

            // Read one label (word until whitespace or semicolon)
            let label = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

            if !label.is_empty() {
                label_storage.store_and_ref(&label);
                count += 1;
            }
        }

        // c) Check that `num_taxa` many labels parsed
        if count != self.num_leaves {
            return Err(ParsingError::invalid_taxa_block(
                &mut self.byte_parser,
                format!(
                    "Number of parsed labels ({}) did not match ntax value ({}).",
                    count, self.num_leaves
                ),
            ));
        }

        Ok(label_storage)
    }

    /// Helps parsing TREES block, responsible for parsing `TRANSLATE` command.
    /// If command exists, returns parsed mapping, `None` otherwise.
    ///
    /// Assumes the parser is positioned at the start of the `TRANSLATE` command
    /// (after any whitespace/comments).
    /// After this method, the parser will be positioned after the semicolon of
    /// the `TRANSLATE` command.
    fn parse_tree_block_translate(
        &mut self,
    ) -> Result<Option<HashMap<String, String>>, ParsingError> {
        // a) Parse "TRANSLATE"
        self.byte_parser.skip_comment_and_whitespace()?;
        if !self.byte_parser.consume_if_sequence(TRANSLATE) {
            // there might be no TRANSLATE command, which is fine if the next command is a TREE
            return if self.byte_parser.peek_is_sequence(TREE) {
                Ok(None)
            } else {
                Err(ParsingError::invalid_taxa_block(
                    &mut self.byte_parser,
                    String::from("Expected 'TRANSLATE' or first 'TREE' in TREES block."),
                ))
            };
        }

        // b) Parse pairs "id/short label"
        let mut map: HashMap<String, String> = HashMap::with_capacity(self.num_leaves);
        loop {
            self.byte_parser.skip_comment_and_whitespace()?;

            // Read key (short label or id)
            let key = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

            // Expect a space
            if !self.byte_parser.consume_if(b' ') {
                return Err(ParsingError::invalid_trees_block(
                    &mut self.byte_parser,
                    String::from("Expected ' ' in between key and label."),
                ));
            }

            // Parse label
            let label = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;
            self.byte_parser.skip_whitespace();

            // d) Add to HashMap
            map.insert(key, label);

            // e) Continue if next is a comma
            if self.byte_parser.consume_if(b',') {
                continue;
            }
            // but stop if semicolon (end of "TRANSLATE" command)
            if self.byte_parser.consume_if(b';') {
                break;
            }
            // and otherwise invalid
            let char = self.byte_parser.peek().unwrap().to_string();
            return Err(ParsingError::invalid_trees_block(
                &mut self.byte_parser,
                format!("Unexpected char '{char}' in TRANSLATE."),
            ));
        }

        // c) small check
        assert_eq!(map.len(), self.num_leaves);

        Ok(Option::from(map))
    }

    /// Helps parsing TREES block, responsible for parsing all `TREE` commands.
    /// Returns all [GenTree]s parsed and with labels resolved.
    ///
    /// Assumes the parser is positioned at the start of the first `TREE` command
    /// (after any whitespace/comments).
    /// After this method, the parser will be positioned after all `TREE` commands,
    /// so before the block closing keyword.
    fn parse_tree_block_trees(&mut self, trees: &mut Vec<T::Tree>) -> Result<(), ParsingError> {
        let newick_parser = &mut self.newick_parser;

        // let mut tree_count = 0;
        loop {
            self.byte_parser.skip_comment_and_whitespace()?;

            // Stop if "END;"
            if self.byte_parser.peek_is_sequence(BLOCK_END) {
                break;
            }

            // Expect "TREE"
            if !self.byte_parser.consume_if_sequence(b"tree") {
                return Err(ParsingError::invalid_trees_block(
                    &mut self.byte_parser,
                    String::from("Expected 'tree' in tree block."),
                ));
            }

            // Parse tree name
            let name = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

            // Expect "="
            self.byte_parser.skip_whitespace();
            if !self.byte_parser.consume_if(b'=') {
                return Err(ParsingError::invalid_trees_block(
                    &mut self.byte_parser,
                    String::from("Expected '=' in tree block. "),
                ));
            }

            // Skip optional "[&R/U]" (rooter or unrooted tree) by considering it a comment
            self.byte_parser.skip_comment_and_whitespace()?;

            // Parse tree
            let tree = newick_parser.parse_str_and_name(&mut self.byte_parser, Some(name))?;

            // Verbose information
            // tree_count += 1;
            // if tree_count % 10 == 0 {
            //     print!(".");
            // }
            // if tree_count % 1000 == 0 {
            //     println!(" ({tree_count})");
            // }

            // Store tree
            trees.push(tree);
        }

        Ok(())
    }

    /// Parses a single TREE entry.
    ///
    /// Assumes the parser is positioned at the start of a `TREE` command
    /// (after any whitespace/comments). After this method, the parser will
    /// be positioned right after the semicolon of this `TREE` command.
    ///
    /// # Returns
    /// * `Ok(Some(Tree))` - Successfully skipped a tree
    /// * `Ok(None)` - No more trees (encountered END;)
    /// * `Err(ParsingError)` - If the format is invalid
    fn parse_single_tree(&mut self) -> Result<Option<T::Tree>, ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        // Check if we've reached the end of the TREES block
        if self.byte_parser.peek_is_sequence(BLOCK_END) {
            return Ok(None);
        }

        // Expect "TREE"
        if !self.byte_parser.consume_if_sequence(TREE) {
            return Err(ParsingError::invalid_trees_block(
                &mut self.byte_parser,
                String::from("Expected 'TREE' in tree command."),
            ));
        }

        // Parse tree name
        let name = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

        // Expect "="
        self.byte_parser.skip_whitespace();
        if !self.byte_parser.consume_if(b'=') {
            return Err(ParsingError::invalid_trees_block(
                &mut self.byte_parser,
                String::from("Expected '=' after tree name in tree command."),
            ));
        }

        // Skip optional "[&R/U]" annotation
        self.byte_parser.skip_comment_and_whitespace()?;

        // Parse the Newick tree
        let tree = self
            .newick_parser
            .parse_str_and_name(&mut self.byte_parser, Some(name))?;
        Ok(Some(tree))
    }

    /// Skips over a single TREE entry without parsing the Newick string.
    ///
    /// Assumes the parser is positioned at the start of a `TREE` command
    /// (after any whitespace/comments). After this method, the parser will be
    /// positioned right after the semicolon of this `TREE` command.
    ///
    /// # Returns
    /// * `Ok(true)` - Successfully skipped a tree
    /// * `Ok(false)` - No more trees (encountered END;)
    /// * `Err(ParsingError)` - If the format is invalid
    fn skip_tree(&mut self) -> Result<bool, ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        // Check if we've reached the end of the TREES block
        if self.byte_parser.peek_is_sequence(BLOCK_END) {
            return Ok(false);
        }

        // Expect "TREE"
        if !self.byte_parser.consume_if_sequence(TREE) {
            return Err(ParsingError::invalid_trees_block(
                &mut self.byte_parser,
                String::from("Expected 'tree' in tree command."),
            ));
        }

        // Skip tree name (consume until '=')
        if !self.byte_parser.consume_until(b'=', Exclusive) {
            return Err(ParsingError::invalid_trees_block(
                &mut self.byte_parser,
                String::from("Expected '=' in tree command."),
            ));
        }
        self.byte_parser.next_byte(); // consume the '='

        // Skip optional whitespace/comments and "[&R/U]" annotation
        self.byte_parser.skip_comment_and_whitespace()?;

        // Skip the Newick string (everything until and including semicolon)
        if !self.byte_parser.consume_until(b';', Inclusive) {
            return Err(ParsingError::unexpected_eof(&mut self.byte_parser));
        }

        Ok(true)
    }

    /// Counts the number of trees in the TREES block without parsing them.
    ///
    /// This method saves the current parser position, counts all trees,
    /// then restores the position to where it was before counting.
    ///
    /// # Returns
    /// The number of trees in the TREES block
    fn count_trees(&mut self) -> Result<usize, ParsingError> {
        // Save current position
        let saved_pos = self.byte_parser.position();

        // Count trees
        let mut count = 0;
        while self.skip_tree()? {
            count += 1;
        }

        // Restore position
        self.byte_parser.set_position(saved_pos);

        Ok(count)
    }
}

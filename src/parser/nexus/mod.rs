///! TODO

mod defs;

use crate::model::leaf_label_map::LeafLabelMap;
use crate::model::tree::Tree;
use crate::parser::byte_parser::ByteParser;
use crate::parser::byte_parser::ConsumeMode::{Exclusive, Inclusive};
use crate::parser::byte_source::{ByteSource, InMemoryByteSource};
use crate::parser::newick::{LabelResolver, NewickParser};
use crate::parser::nexus::defs::*;
use crate::parser::parsing_error::ParsingError;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

// =#========================================================================#=
// PARSING MODE
// =#========================================================================#=
/// Mode of [NexusParser] when parsing the TREES block:
enum TreeParsingMode {
    /// Eagerly parse all trees upfront and store them
    Eager { trees: Vec<Tree> },
    /// Lazily parse trees as requested without storing them
    Lazy {
        /// Byte position where the first tree to parse begins (for reset)
        start_byte_pos: usize
    },
}


// =#========================================================================#=
// BURNIN
// =#========================================================================#=
/// Specifies how many initial trees to skip as burnin.
///
/// burnin is commonly used in MCMC sampling to discard initial trees
/// before the chain has converged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Burnin {
    /// Skip a fixed number of trees.
    ///
    /// # Example
    /// ```
    /// use nexus_parser::parser::nexus::Burnin;
    /// let burnin = Burnin::Count(1001); // Skip first 1001 trees
    /// ```
    Count(usize),

    /// Skip a percentage of total trees.
    ///
    /// The percentage must be in the range [0.0, 1.0);
    /// behaviour undefined otherwise.
    ///
    /// # Example
    /// ```
    /// use nexus_parser::parser::nexus::Burnin;
    /// // Skip first 25% of trees
    /// let burnin = Burnin::Percentage(0.25);
    /// ```
    Percentage(f64),
}

impl Burnin {
    /// Calculates the absolute number of trees to skip given the total tree count.
    ///
    /// # Arguments
    /// * `total_trees` - Total number of trees in the file
    ///
    /// # Returns
    /// The number of trees to skip as burnin
    pub(crate) fn to_count(&self, num_total_trees: usize) -> usize {
        match self {
            Burnin::Count(n) => *n,
            Burnin::Percentage(pct) => (num_total_trees as f64 * pct).floor() as usize,
        }
    }

    // TODO
    pub(crate) fn significant(&self) -> bool {
        match &self {
            Burnin::Count(n) => { *n >= 100 } // TODO make config constants
            Burnin::Percentage(p) => { *p >= 0.05 }
        }
    }
}


// =#========================================================================#=
// NEXUS PARSER BUILDER
// =#========================================================================#=
/// Builder for configuring and creating a [NexusParser].
///
/// This builder provides an API for configuring how NEXUS files should be parsed
/// before initialization. Once configured, call [build()](NexusParserBuilder::build)
/// to create an initialized [NexusParser] ready for tree retrieval.
///
/// # Configuration Options
///
/// * **Parsing mode**: Choose between eager (parse all trees upfront) or lazy (parse on-demand)
///   - [eager()](NexusParserBuilder::eager) - Parse and store all trees during initialization (default)
///   - [lazy()](NexusParserBuilder::lazy) - Parse trees one at a time as requested
///
/// * **Skip first**: Skip the first tree (some software creates files with e.g. 10001 trees samples)
///   - [with_skip_first()](NexusParserBuilder::with_skip_first) - Skip the very first tree
///
/// * **burnin**: Discard some initial trees as burnin (commonly used for MCMC samples)
///   - [with_burnin()](NexusParserBuilder::with_burnin) - Skip a fixed count or percentage
///
/// # Example
/// ```ignore
/// use std::fs::File;
/// use nexus_parser::parser::nexus::{NexusParserBuilder, Burnin};
///
/// // Parse a NEXUS file with 10% burnin, eagerly loading all trees
/// let file = File::open("passeriformes.trees")?;
/// let mut parser = NexusParserBuilder::for_file(file)?
///     .with_burnin(Burnin::Percentage(0.1))
///     .eager()
///     .build()?;
///
/// // Iterate through trees
/// while let Some(tree) = parser.next_tree()? {
///     println!("Tree height: {}", tree.height());
/// }
/// ```
///
/// # Lazy vs Eager Mode
///
/// **Eager mode** (default):
/// - All trees are parsed during [build()](NexusParserBuilder::build) and stored in memory
/// - Fast iteration with [next_tree()](NexusParser::next_tree()) or retrieve all
/// - Higher memory usage for large tree sets
/// - Best for: small-to-medium datasets, analyzing all trees multiple times
///
/// **Lazy mode**:
/// - Trees are parsed on-demand when calling [next_tree()](NexusParser::next_tree())
/// - Lower memory footprint
/// - Cannot reset or iterate multiple times without reparsing
/// - Best for: streaming large datasets, single-pass analysis
pub struct NexusParserBuilder<S: ByteSource> {
    mode: TreeParsingMode,
    source: S,
    burnin: Burnin,
    skip_first: bool,
}

// ============================================================================
// Building - InMemory-ByteSource specific (pub)
// ============================================================================
impl NexusParserBuilder<InMemoryByteSource> {
    /// Creates a new builder from a file.
    ///
    /// The entire file is read into memory and wrapped in an `InMemoryByteSource`.
    /// The builder is initialized with default settings:
    /// - Eager parsing mode
    /// - First tree not skipped
    /// - No burnin (all trees included)
    ///
    /// # Arguments
    /// * `file` - The file handle to read from
    ///
    /// # Returns
    /// A builder configured with defaults, ready for method chaining
    ///
    /// # Errors
    /// Returns an I/O error if the file cannot be read
    ///
    /// # Example
    /// ```ignore
    /// use std::fs::File;
    /// use nexus_parser::parser::nexus::{NexusParserBuilder, Burnin};
    ///
    /// let file = File::open("falconidae.trees")?;
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .with_burnin(Burnin::Count(1000))
    ///     .with_skip_first()
    ///     .build()?;
    /// ```
    pub fn for_file(mut file: File) -> std::io::Result<Self> {
        // Read entire file into memory
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let source = InMemoryByteSource::from_vec(contents);

        Ok(NexusParserBuilder {
            mode: TreeParsingMode::Eager { trees: Vec::new() },
            source,
            burnin: Burnin::Count(0),
            skip_first: false,
        })
    }
}

// ============================================================================
// Building - Generic (pub)
// ============================================================================
impl<S: ByteSource> NexusParserBuilder<S> {
    /// Configure the parser to use **eager mode** (parse all trees upfront).
    ///
    /// In eager mode, all trees are parsed and stored in memory during the
    /// [build()](NexusParserBuilder::build) call. This allows fast iteration
    /// and multiple passes through the trees, but uses more memory for large
    /// datasets.
    ///
    /// This is the **default mode**.
    ///
    /// # Returns
    /// The builder with eager mode configured
    ///
    /// # Example
    /// ```ignore
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .eager()  // Parse all trees during build()
    ///     .build()?;
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
    /// ```ignore
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .lazy()  // Parse trees on-demand
    ///     .build()?;
    ///
    /// // Single pass through trees
    /// while let Some(tree) = parser.next_tree()? {
    ///     // Process tree
    /// }
    /// ```
    pub fn lazy(mut self) -> Self {
        self.mode = TreeParsingMode::Lazy { start_byte_pos: 0 };
        self
    }

    /// Configure burnin, i.e., discard/skip initial trees.
    ///
    /// burnin skips trees from the beginning of the file, either as a fixed count
    /// or as a percentage of total trees. This is commonly used in Bayesian phylogenetics
    /// to discard samples before the MCMC chain has converged.
    ///
    /// If both burnin and [with_skip_first()](NexusParserBuilder::with_skip_first)
    /// are configured, the first tree is skipped, then burnin is applied to
    /// the remaining trees.
    ///
    /// # Arguments
    /// * `burnin` - The burnin specification ([Burnin::Count] or [Burnin::Percentage])
    ///
    /// # Returns
    /// The builder with burnin configured
    ///
    /// # Example
    /// ```ignore
    /// use nexus_parser::parser::nexus::Burnin;
    ///
    /// // Skip first 1000 trees
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .with_burnin(Burnin::Count(1000))
    ///     .build()?;
    ///
    /// // Skip first 1% of trees
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .with_burnin(Burnin::Percentage(0.01))
    ///     .build()?;
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
    /// [with_burnin()](NexusParserBuilder::with_burnin) are configured, the first tree
    /// is skipped, then burnin is applied to the remaining trees.
    ///
    /// # Returns
    /// The builder with skip-first configured
    ///
    /// # Example
    /// ```ignore
    /// // Skip first tree (often a consensus tree)
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .with_skip_first()
    ///     .build()?;
    ///
    /// // Skip first tree AND apply 3% burnin to the rest
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .with_skip_first()
    ///     .with_burnin(Burnin::Percentage(0.03))
    ///     .build()?;
    /// ```
    pub fn with_skip_first(mut self) -> Self {
        self.skip_first = true;
        self
    }

    /// Builds and initializes the [NexusParser] with the configured settings.
    ///
    /// This method:
    /// 1. Creates the parser with the configured options
    /// 2. Parses the NEXUS header and TAXA block
    /// 3. Parses the TRANSLATE command (if present) in the TREES block
    /// 4. Counts total trees and applies burnin/skip-first settings
    /// 5. In eager mode: parses and stores all remaining trees
    /// 6. In lazy mode: positions the parser at the first tree to return
    ///
    /// After this returns successfully, the parser is ready to retrieve trees
    /// via [next_tree()](NexusParser::next_tree) (recommended for lazy mode)
    /// or [into_results()](NexusParser::into_results) (recommended for eager mode).
    ///
    /// # Returns
    /// An initialized [NexusParser] ready for tree retrieval
    ///
    /// # Errors
    /// Returns a [ParsingError] if:
    /// - The file is not a valid NEXUS file
    /// - Required blocks (TAXA, TREES) are missing
    /// - The NEXUS format is malformed
    /// - Tree parsing fails (in eager mode)
    ///
    /// # Example
    /// ```ignore
    /// let parser = NexusParserBuilder::for_file(file)?
    ///     .with_burnin(Burnin::Percentage(0.1))
    ///     .eager()
    ///     .build()?;  // Parses NEXUS structure and all trees
    ///
    /// // Parser is now ready
    /// println!("Found {} trees", parser.num_trees());
    /// ```
    pub fn build(self) -> Result<NexusParser<S>, ParsingError> {
        let mut nexus_parser = NexusParser {
            mode: self.mode,
            newick_parser: NewickParser::new(),
            byte_parser: ByteParser::new(self.source),
            num_leaves: 0,
            num_total_trees: 0,
            num_trees: 0,
            start_tree_pos: 0,
            tree_pos: 0,
            burnin: self.burnin,
            skip_first: self.skip_first,
        };

        nexus_parser.init()?;
        Ok(nexus_parser)
    }
}


// =#========================================================================#=
// NEXUS PARSER
// =#========================================================================#=
/// Parser for NEXUS phylogenetic tree files.
///
/// This parser provides access to phylogenetic trees stored in NEXUS format files,
/// which are commonly used in Bayesian phylogenetics (e.g., from BEAST2, MrBayes, RevBayes).
/// A NEXUS file typically contains:
/// - A TAXA block defining the species/labels
/// - A TREES block containing multiple phylogenetic trees
/// - Optional TRANSLATE commands mapping short keys to full taxon labels
///
/// # Construction
///
/// Use [NexusParserBuilder] to configure and create a parser:
///
/// ```ignore
/// use nexus_parser::parser::nexus::{NexusParserBuilder, Burnin};
/// use std::fs::File;
///
/// let file = File::open("phylo.nex")?;
/// let mut parser = NexusParserBuilder::for_file(file)?
///     .with_burnin(Burnin::Percentage(0.25))
///     .eager()
///     .build()?;
/// ```
///
/// # Usage
///
/// After initialization via the builder, use the parser to:
/// - Iterate through trees with [next_tree()](NexusParser::next_tree)
/// - Query metadata like [num_trees()](NexusParser::num_trees), [num_leaves()](NexusParser::num_leaves)
/// - Access the taxon label map with [leaf_label_map()](NexusParser::leaf_label_map)
/// - Extract all results with [into_results()](NexusParser::into_results)
///
/// # Example
///
/// ```ignore
/// use std::fs::File;
/// use nexus_parser::parser::nexus::{NexusParserBuilder, Burnin};
///
/// let file = File::open("aves.trees")?;
/// let mut parser = NexusParserBuilder::for_file(file)?
///     .with_burnin(Burnin::Percentage(0.1))
///     .build()?;
///
/// println!("Loaded {} taxa", parser.num_leaves());
/// println!("Analyzing {} trees (after burnin)", parser.num_trees());
///
/// // Process each tree
/// while let Some(tree) = parser.next_tree()? {
///     println!("Tree height: {:.4}", tree.height());
/// }
///
/// // Or extract everything at once
/// let (trees, labels) = parser.into_results()?;
/// ```
///
/// # Assumptions
/// * A `TAXA` and a `TREES` block are present, in this order
/// * A `TRANSLATE` command, if present, precedes any `TREE` command, with following details:
///   - Command is a comma seperated list of pairs of "id/short label":
///         `TRANSLATE [<key1=short1/id1> <label1>, ...];`
///   - Mapping should *consistently* use integer or shorts as key; behaviour undefined otherwise
///   - `<label>` must match a label provided in `TAXA` blog.
///   - Length of mapping must match number of taxa/labels.
///         (This is a program specific requirement, not of NEXUS files.)
///   - A label with a space in it must be enclosed in single quotes and ...
///   - A label with an apostrophe in it must be enclosed in single quotes
///     and the apostrophe must be escaped with an apostrophe/single quote:
///     e.g. `Wilson's Storm-petrel` becomes `'Wilson''s storm-petrel'`
///   - No comments within pair allowed, only between comma and next pair,
///     e.g. `[cool seabird] stormy 'Wilson''s_storm-petrel',`
/// * Trees come in semicolon separated list of tree commands
/// * One tree command has format `tree <name> = <Newick string>;`
///   - Each pair is separated by a comma, optional whitespace and comments
///   - Only one mapping per taxon allowed
///   - Same label rules apply
///
pub struct NexusParser<S: ByteSource> {
    /// Mode to parse trees
    mode: TreeParsingMode,
    /// Continuously used to parse Newick strings, including resolving labels
    newick_parser: NewickParser,
    /// Accessor to the underlying bytes/file being parsed
    byte_parser: ByteParser<S>,

    /// Whether to skip the first tree
    skip_first: bool,
    /// Amount of burnin to discard/skip
    burnin: Burnin,

    /// Number of leaves/taxa in all TAXA block and all trees (must be consistent)
    num_leaves: usize,

    /// The total number of `TREE` commands in the Nexus file
    num_total_trees: usize,
    /// The number of [Tree]/`TREE` commands considers afters skipped/discarded ones
    /// - Invariant: `num_trees <= num_total_trees`
    num_trees: usize,
    /// The first [Tree]/`TREE` command to consider (0-indexed)
    /// - Invariant: `num_trees + start_tree_pos = num_total_trees`
    start_tree_pos: usize,
    /// Position of currently next [Tree]/`TREE` commands to consider for `next_tree()`
    /// - Invariant: `start_tree_pos <= tree_pos < num_total_trees`
    tree_pos: usize,
}

// ============================================================================
// Initialization & State (private)
// ============================================================================
impl<S: ByteSource> NexusParser<S> {
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
        let label_map = self.parse_taxa_block()?;

        // > TREES block
        // Skip until TREES block and ...
        self.skip_until_block(NexusBlock::Trees)?;
        // ... handle TRANSLATE command
        let map = self.parse_tree_block_translate()?;

        // ... and based on whether it exists, pick the appropriate label resolver
        let resolver = self.choose_resolver(label_map, map)?;
        self.newick_parser = NewickParser::new().with_num_leaves(self.num_leaves).with_resolver(resolver);

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
                all_trees.into_iter()
                    .skip(self.start_tree_pos)
                    .collect()
            };

            self.mode = TreeParsingMode::Eager { trees };
        }

        Ok(())
    }

    /// Helper method to configure tree count fields based on total tree count.
    ///
    /// Sets `num_total_trees`, `num_trees`, `start_tree_pos`, and `tree_pos` based on
    /// `skip_first` and burnin configuration.
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
        skip_count += self.burnin.to_count(num_total_trees - skip_count);

        // The actual number of trees that should be parsed (considered):
        self.num_trees = num_total_trees.saturating_sub(skip_count);

        // The position where actual trees start (after skipped and burnin trees)
        self.start_tree_pos = skip_count;
        self.tree_pos = skip_count;
    }

    /// Helper method to pick and configure the right [LabelResolver] at initialization.
    fn choose_resolver(&mut self, label_map: LeafLabelMap, map: Option<HashMap<String, String>>) -> Result<LabelResolver, ParsingError> {
        Ok(match map {
            None => {
                LabelResolver::new_verbatim_labels_resolver(label_map)
            }
            Some(map) => {
                // Assert that labels match those provided in TAXA block
                if !label_map.check_consistency_with_translation(&map) {
                    return Err(ParsingError::invalid_translate_command(&self.byte_parser));
                }

                // Check if all keys are integers to use the more efficient NexusIntegerLabels resolver
                let all_keys_are_integers = map.keys().all(|key| key.parse::<usize>().is_ok());

                if all_keys_are_integers {
                    LabelResolver::new_nexus_integer_labels_resolver(map, label_map)
                } else {
                    LabelResolver::new_nexus_labels_resolver(map, label_map)
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
// Deconstruction (pub)
// ============================================================================
impl<S: ByteSource> NexusParser<S> {
    /// Consumes this [NexusParser] and returns the build [LeafLabelMap].
    ///
    /// This extracts the final underlying label-to-index mapping,
    /// regardless of the configuration.
    ///
    /// # Returns
    /// The [LeafLabelMap] based on the parsed Nexus file.
    pub fn into_leaf_label_map(self) -> LeafLabelMap {
        self.newick_parser.into_leaf_label_map()
    }

    /// Consumes this [NexusParser] and returns the resulting [Tree]s and [LeafLabelMap].
    ///
    /// This parses all trees and extracts the final underlying label-to-index mapping,
    /// regardless of the configuration.
    ///
    /// # Returns
    /// A vector of the [Tree]s and the corresponding [LeafLabelMap] in the parsed Nexus file.
    pub fn into_results(mut self) -> Result<(Vec<Tree>, LeafLabelMap), ParsingError> {
        match self.mode {
            TreeParsingMode::Eager { trees } => {
                return Ok((trees, self.newick_parser.into_leaf_label_map()));
            }
            TreeParsingMode::Lazy { .. } => {
                let mut all_trees = Vec::new();
                self.reset();
                while let Some(tree) = self.next_tree()? {
                    all_trees.push(tree);
                }
                return Ok((all_trees, self.newick_parser.into_leaf_label_map()));
            }
        }
    }
}

// ============================================================================
// Getters / Accessors, etc. (pub)
// ============================================================================
impl<S: ByteSource> NexusParser<S> {
    /// Get the number of leaves/taxa based on TAXA block
    pub fn num_leaves(&self) -> usize {
        self.num_leaves
    }

    /// Get ref to [LeafLabelMap] of all taxa based on TAXA block
    pub fn leaf_label_map(&self) -> &LeafLabelMap {
        &self.newick_parser.leaf_label_map()
    }

    /// Get the number of trees (without burnin trees)
    pub fn num_trees(&mut self) -> usize {
        self.num_trees
    }

    /// Get the total number of trees including skipped+burnin
    pub fn num_total_trees(&mut self) -> usize {
        self.num_total_trees
    }

    /// Get the next tree, intended for use in lazy mode.
    ///
    /// # Returns
    /// * `Ok(Tree)` - The next [Tree] parsed from the file, if in lazy mode
    /// * `Ok(Tree (.clone()))` - A clone of the next [Tree] parsed from the file, if in eager mode
    /// * `Ok(None)` - If there is no other tree
    /// * [ParsingError] - If something went wrong parsing the next tree
    pub fn next_tree(&mut self) -> Result<Option<Tree>, ParsingError> {
        match &self.mode {
            TreeParsingMode::Eager { trees } => {
                if self.tree_pos < self.start_tree_pos + self.num_trees {
                    let tree = trees[self.tree_pos - self.start_tree_pos].clone();
                    self.tree_pos += 1;
                    Ok(Some(tree))
                } else {
                    Ok(None)
                }
            }
            TreeParsingMode::Lazy { .. } => {
                // Check if we've reached the end
                if self.tree_pos >= self.start_tree_pos + self.num_trees {
                    return Ok(None);
                }

                // Parse next tree on demand
                let tree = self.parse_single_tree()?;
                if tree.is_none() {
                    return Err(ParsingError::unexpected_eof(&self.byte_parser));
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
impl<S: ByteSource> NexusParser<S> {
    /// Parses header `#NEXUS` at start of file or returns `ParsingError::MissingNexusHeader` otherwise.
    fn parse_nexus_header(&mut self) -> Result<(), ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        if !self.byte_parser.consume_if_sequence(NEXUS_HEADER) {
            return Err(ParsingError::missing_nexus_header(&self.byte_parser));
        }

        Ok(())
    }

    /// Skips Nexus blocks until we encounter the target block type, whose header is consumed;
    /// returns `[ParsingError::UnexpectedEOF]` if not found.
    fn skip_until_block(&mut self, target: NexusBlock) -> Result<(), ParsingError> {
        loop {
            if self.byte_parser.is_eof() {
                return Err(ParsingError::unexpected_eof(&self.byte_parser));
            }

            let block_type = self.detect_next_block()?;

            if block_type == target {
                return Ok(());
            }

            self.skip_to_block_end()?;
        }
    }

    /// Detects the next Nexus block, which must start with header `BEGIN <BlockType>;` (case-insensitive),
    /// consumes its header, and returns its BlockType, or a ParsingError if something went wrong.
    fn detect_next_block(&mut self) -> Result<NexusBlock, ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        if !self.byte_parser.consume_if_sequence(BLOCK_BEGIN) {
            return Err(ParsingError::invalid_formatting(&self.byte_parser));
        }
        self.byte_parser.skip_comment_and_whitespace()?;

        let start_pos = self.byte_parser.position();
        if !self.byte_parser.consume_until(b';', Exclusive) {
            return Err(ParsingError::unexpected_eof(&self.byte_parser));
        }

        let block_name = std::str::from_utf8(self.byte_parser.slice_from(start_pos))
            .map_err(|_| ParsingError::invalid_block_name(&self.byte_parser))?
            .to_string();

        self.byte_parser.next(); // consume the ';' now (already know that this is next byte)

        Ok(NexusBlock::from_name(&block_name))
    }

    /// Skips block, e.g. continuing until encountering `END;`.
    fn skip_to_block_end(&mut self) -> Result<(), ParsingError> {
        if !self.byte_parser.consume_until_sequence(BLOCK_END, Inclusive) {
            return Err(ParsingError::unexpected_eof(&self.byte_parser));
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
    /// Return [ParsingError::UnexpectedEOF] if block, command, or comment not properly closed,
    /// and [ParsingErrror::InvalidTaxaBlock] if commands not encountered in expected order (as specified above).
    fn parse_taxa_block(&mut self) -> Result<LeafLabelMap, ParsingError> {
        // 1. Parse number of taxa command "DIMENSIONS NTAX=n;"
        self.parse_taxa_block_ntax()?;

        // 2. Parse list of taxa labels in `TAXLABEL` command, including consuming closing ";"
        let label_map = self.parse_taxa_block_labels()?;

        // 3. Move to end of block
        self.skip_to_block_end()?;
        // Would expect only "END;" besides whitespace and comments, but not enforced here

        Ok(label_map)
    }

    /// Helps parsing TAXA block, responsible for the `ntax` command
    /// and returning the result: the number of taxa.
    fn parse_taxa_block_ntax(&mut self) -> Result<(), ParsingError> {
        // This could be a replaced with a general parsing structure for such a type of command,
        // but for our nexus files we only have this one.
        // a) Parse "DIMENSIONS NTAX="
        self.byte_parser.skip_comment_and_whitespace()?;
        if !self.byte_parser.consume_if_sequence(DIMENSIONS) {
            return Err(ParsingError::invalid_taxa_block(&self.byte_parser,
                String::from("Expected 'DIMENSIONS' in TAXA block.")));
        }

        self.byte_parser.skip_whitespace();
        if !self.byte_parser.consume_if_sequence(NTAX) {
            return Err(ParsingError::invalid_taxa_block(&self.byte_parser,
                String::from("Expected 'NTAX' in TAXA block.")));
        }

        self.byte_parser.skip_whitespace();
        if !self.byte_parser.consume_if(b'=') {
            return Err(ParsingError::invalid_taxa_block(&self.byte_parser,
                String::from("Expected '=' in TAXA block.")));
        }

        // b) Read the number `n` and consume ";"
        self.byte_parser.skip_whitespace();
        let start_pos = self.byte_parser.position();
        if !self.byte_parser.consume_until(b';', Exclusive) {
            return Err(ParsingError::unexpected_eof(&self.byte_parser));
        }
        let ntax_str = std::str::from_utf8(self.byte_parser.slice_from(start_pos))
            .map_err(|_| ParsingError::invalid_taxa_block(&self.byte_parser,
                String::from("Invalid UTF-8 in ntax value")))?
            .trim();
        let ntax: usize = ntax_str.parse()
            .map_err(|_| ParsingError::invalid_taxa_block(&self.byte_parser,
                format!("Cannot parse `ntax` value: {}", ntax_str)))?;
        self.byte_parser.next(); // consume the semicolon

        self.num_leaves = ntax;
        Ok(())
    }

    /// Helps parsing TAXA block, responsible for the `TAXLABEL` command
    /// and returning the parsed taxa as [LeafLabelMap].
    fn parse_taxa_block_labels(&mut self) -> Result<LeafLabelMap, ParsingError> {
        // a) Parse "TAXLABELS"
        self.byte_parser.skip_comment_and_whitespace()?;
        if !self.byte_parser.consume_if_sequence(TAXLABELS) {
            return Err(ParsingError::invalid_taxa_block(&self.byte_parser,
                String::from("Expected 'TAXLABELS' in TAXA block.")));
        }

        // b) Read labels until semicolon
        let mut label_map = LeafLabelMap::new(self.num_leaves);
        loop {
            self.byte_parser.skip_comment_and_whitespace()?;

            // Stop once encountering semicolon (end of labels command)
            if self.byte_parser.peek() == Some(b';') {
                self.byte_parser.next();
                break;
            }

            // Read one label (word until whitespace or semicolon)
            let label = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

            if !label.is_empty() {
                label_map.insert(label);
            }
        }

        // c) Check that `num_taxa` many labels parsed
        if !label_map.is_full() {
            return Err(ParsingError::invalid_taxa_block(
                &self.byte_parser,
                format!("Number of parsed labels ({}) did not match ntax value ({}).",
                    label_map.num_labels(), self.num_leaves)));
        }

        Ok(label_map)
    }

    /// Helps parsing TREES block, responsible for parsing `TRANSLATE` command.
    /// If command exists, returns parsed mapping, `None` otherwise.
    ///
    /// Assumes the parser is positioned at the start of the `TRANSLATE` command (after any whitespace/comments).
    /// After this method, the parser will be positioned after the semicolon of the `TRANSLATE` command.
    fn parse_tree_block_translate(&mut self) -> Result<Option<HashMap<String, String>>, ParsingError> {
        // a) Parse "TRANSLATE"
        self.byte_parser.skip_comment_and_whitespace()?;
        if !self.byte_parser.consume_if_sequence(TRANSLATE) {
            // there might be no TRANSLATE command, which is fine if the next command is a TREE
            return if self.byte_parser.peek_is_sequence(TREE) {
                Ok(None)
            } else {
                Err(ParsingError::invalid_taxa_block(&self.byte_parser, String::from("Expected 'TRANSLATE' or first 'TREE' in TREES block.")))
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
                return Err(ParsingError::invalid_trees_block(&self.byte_parser, String::from("Expected ' ' in between key and label.")));
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
            return Err(ParsingError::invalid_trees_block(&self.byte_parser, format!("Unexpected char '{char}' in TRANSLATE.")));
        }


        // c) small check
        assert_eq!(map.len(), self.num_leaves);

        Ok(Option::from(map))
    }

    /// Helps parsing TREES block, responsible for parsing all `TREE` commands.
    /// Returns all [Tree]s parsed and with labels resolved.
    ///
    /// Assumes the parser is positioned at the start of the first `TREE` command
    /// (after any whitespace/comments).
    /// After this method, the parser will be positioned after all `TREE` commands,
    /// so before the block closing keyword.
    fn parse_tree_block_trees(&mut self, trees: &mut Vec<Tree>) -> Result<(), ParsingError> {
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
                return Err(ParsingError::invalid_trees_block(&self.byte_parser, String::from("Expected 'tree' in tree block.")));
            }

            // Parse tree name
            let name = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

            // Expect "="
            self.byte_parser.skip_whitespace();
            if !self.byte_parser.consume_if(b'=') {
                return Err(ParsingError::invalid_trees_block(&self.byte_parser, String::from("Expected '=' in tree block. ")));
            }

            // Skip optional "[&R/U]" (rooter or unrooted tree) by considering it a comment
            self.byte_parser.skip_comment_and_whitespace()?;

            // Parse tree
            let tree = newick_parser.parse(&mut self.byte_parser)?.with_name(name);

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

    /// Skips over a single TREE entry without parsing the Newick string.
    ///
    /// Assumes the parser is positioned at the start of a `TREE` command (after any whitespace/comments).
    /// After this method, the parser will be positioned right after the semicolon of this `TREE` command.
    ///
    /// # Returns
    /// * `Ok(Tree)` - Successfully skipped a tree
    /// * `Ok(false)` - No more trees (encountered END;)
    /// * `Err(ParsingError)` - If the format is invalid
    fn parse_single_tree(&mut self) -> Result<Option<Tree>, ParsingError> {
        self.byte_parser.skip_comment_and_whitespace()?;

        // Check if we've reached the end of the TREES block
        if self.byte_parser.peek_is_sequence(BLOCK_END) {
            return Ok(None);
        }

        // Expect "TREE"
        if !self.byte_parser.consume_if_sequence(TREE) {
            return Err(ParsingError::invalid_trees_block(&self.byte_parser,
                String::from("Expected 'TREE' in tree command.")));
        }

        // Parse tree name
        let name = self.byte_parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

        // Expect "="
        self.byte_parser.skip_whitespace();
        if !self.byte_parser.consume_if(b'=') {
            return Err(ParsingError::invalid_trees_block(&self.byte_parser,
                String::from("Expected '=' after tree name in tree command.")));
        }

        // Skip optional "[&R/U]" annotation
        self.byte_parser.skip_comment_and_whitespace()?;

        // Parse the Newick tree
        Ok(Some(self.newick_parser.parse(&mut self.byte_parser)?.with_name(name)))
    }

    /// Skips over a single TREE entry without parsing the Newick string.
    ///
    /// Assumes the parser is positioned at the start of a `TREE` command (after any whitespace/comments).
    /// After this method, the parser will be positioned right after the semicolon of this `TREE` command.
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
            return Err(ParsingError::invalid_trees_block(&self.byte_parser, String::from("Expected 'tree' in tree command.")));
        }

        // Skip tree name (consume until '=')
        if !self.byte_parser.consume_until(b'=', Exclusive) {
            return Err(ParsingError::invalid_trees_block(&self.byte_parser, String::from("Expected '=' in tree command.")));
        }
        self.byte_parser.next(); // consume the '='

        // Skip optional whitespace/comments and "[&R/U]" annotation
        self.byte_parser.skip_comment_and_whitespace()?;

        // Skip the Newick string (everything until and including semicolon)
        if !self.byte_parser.consume_until(b';', Inclusive) {
            return Err(ParsingError::unexpected_eof(&self.byte_parser));
        }

        Ok(true)
    }

    /// Counts the number of trees in the TREES block without parsing them.
    ///
    /// This method saves the current parser position, counts all trees, then restores
    /// the position to where it was before counting.
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
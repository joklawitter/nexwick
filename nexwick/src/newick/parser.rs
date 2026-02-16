//! Structs and logic to parse Newick strings.
//!
//! This module provides the [NewickParser] struct, which offers methods
//! to parse files or single strings, as well as lazy parsing via a
//! [NewickIterator].

use crate::model::simple_tree_builder::{SimpleLabelStorage, SimpleTreeBuilder};
use crate::model::tree_builder::TreeBuilder;
use crate::model::{CompactTreeBuilder, LabelResolver, LeafLabelMap};
use crate::newick::defs::{DEFAULT_NUM_LEAVES_GUESS, NEWICK_LABEL_DELIMITERS};
use crate::parser::byte_parser::ByteParser;
use crate::parser::byte_source::ByteSource;
use crate::parser::parsing_error::ParsingError;

// =#========================================================================#=
// NEWICK PARSER
// =#========================================================================$=
/// Parser (configuration) for single/multiple Newick format (binary)
/// phylogenetic trees.
///
/// Generic over [TreeBuilder] (construction). Uses a [LabelResolver]
/// (with in turn uses the builders [LabelStorage](crate::model::LabelStorage)
/// to resolve any mapping, e.g. as necessary when parsing a Nexus file with a
/// `TRANSLATE` command.
///
/// # Construction
/// * [`new(tree_builder, resolver)`](Self::new) — generic constructor
/// * [`new_compact_defaults()`](Self::new_compact_defaults)
///     - uses [CompactTreeBuilder] and a [LeafLabelMap] with
///       verbatim label resolution.
/// * [`new_simple_defaults()`](Self::new_simple_defaults)
///     - uses [SimpleTreeBuilder] and a [SimpleLabelStorage] with
///       verbatim label resolution.
///
/// # Configuration
/// * `with_num_leaves(num_leaves)`
///     - Can be configured with number of leaves in trees to parse,
///       otherwise it is inferred from the first parsed tree and then stored.
/// * Plan to include in the future `with_annotations()`, so that it can be configured
///   to parse vertex annotation instead of considering them comments,
///   (e.g. extract `pop_size` and value from "A[&pop_size=0.123]").
///   For now, annotation remains unsupported.
///
/// # Parsing
/// * [`parse_str`](Self::parse_str) — Parse single tree
/// * [`parse_all`](Self::parse_all) — Parse all trees eagerly
/// * [`into_iter`](Self::into_iter) — Parse trees lazily
///
/// # Example
/// ```
/// use nexwick::newick::NewickParser;
/// use nexwick::parser::byte_parser::ByteParser;
///
/// let input = "((A_meleagrides:1.0,A_vulturinum:1.0):0.5,(N_meleagris:1.0,G_plumifera:1.0):0.5);";
/// let mut byte_parser = ByteParser::for_str(input);
/// let mut newick_parser = NewickParser::new_compact_defaults();
///
/// let tree = newick_parser.parse_str(&mut byte_parser).unwrap();
/// let labels = newick_parser.into_label_storage();
/// ```
pub struct NewickParser<T: TreeBuilder> {
    know_num_leaves: bool,
    num_leaves: usize,
    tree_builder: T,
    resolver: LabelResolver<T::Storage>,
}

// ============================================================================
// Construction & Configuration, Deconstruction (pub)
// ============================================================================
impl<T: TreeBuilder> NewickParser<T> {
    /// Creates a new [NewickParser] with the given tree builder
    /// and verbatim label resolver as default.
    pub fn new(tree_builder: T) -> Self {
        let storage = T::create_storage(DEFAULT_NUM_LEAVES_GUESS);
        let resolver = LabelResolver::new_verbatim_labels_resolver(storage);
        Self {
            know_num_leaves: false,
            num_leaves: DEFAULT_NUM_LEAVES_GUESS,
            tree_builder,
            resolver,
        }
    }

    /// Sets the expected number of leaves in each parsed tree.
    ///
    /// This allows pre-allocation of data structures for better performance.
    /// If not set, the parser will count leaves during parsing.
    pub fn with_num_leaves(mut self, num_leaves: usize) -> Self {
        self.num_leaves = num_leaves;
        self.know_num_leaves = true;
        self
    }

    /// Sets the expected number of leaves in each parsed tree.
    ///
    /// This allows pre-allocation of data structures for better performance.
    /// If not set, the parser will count leaves during parsing.
    pub(crate) fn set_num_leaves(&mut self, num_leaves: usize) -> &mut Self {
        self.num_leaves = num_leaves;
        self.know_num_leaves = true;
        self
    }

    /// Replaces the resolver with a custom one.
    ///
    /// Used by [NexusParser](crate::nexus::NexusParser) to provide resolvers
    /// configured from TRANSLATE blocks.
    pub fn with_resolver(mut self, resolver: LabelResolver<T::Storage>) -> Self {
        self.resolver = resolver;
        self
    }

    /// Replaces the resolver with a custom one.
    ///
    /// Used by [NexusParser](crate::nexus::NexusParser) to provide resolvers
    /// configured from TRANSLATE blocks.
    pub(crate) fn set_resolver(&mut self, resolver: LabelResolver<T::Storage>) -> &mut Self {
        self.resolver = resolver;
        self
    }

    /// Consumes the parser and returns the tree builder and resolver.
    pub fn into_parts(self) -> (T, LabelResolver<T::Storage>) {
        (self.tree_builder, self.resolver)
    }

    /// Consumes the parser and returns the underlying
    /// [LabelStorage](crate::model::LabelStorage).
    ///
    /// This should be called after all trees have been parsed to retrieve
    /// the mapping of leaf labels to indices.
    pub fn into_label_storage(self) -> T::Storage {
        self.resolver.into_label_storage()
    }

    /// Get ref to [LabelStorage](crate::model::LabelStorage)
    /// of underlying [LabelResolver]
    pub fn label_storage(&self) -> &T::Storage {
        self.resolver.label_storage()
    }
}

// Convenience Default 1
impl NewickParser<CompactTreeBuilder> {
    /// Creates a new [NewickParser] for [CompactTree](crate::CompactTree)
    /// with default settings:
    /// - Number of leaves is unknown (will be counted during parsing)
    /// - Verbatim label resolution
    pub fn new_compact_defaults() -> Self {
        Self {
            know_num_leaves: false,
            num_leaves: DEFAULT_NUM_LEAVES_GUESS,
            tree_builder: CompactTreeBuilder::new(),
            resolver: LabelResolver::VerbatimLabels(LeafLabelMap::new(DEFAULT_NUM_LEAVES_GUESS)),
        }
    }
}

impl Default for NewickParser<CompactTreeBuilder> {
    fn default() -> Self {
        Self::new_compact_defaults()
    }
}

// Convenience Default 2
impl NewickParser<SimpleTreeBuilder> {
    /// Creates a new [NewickParser] for [SimpleTree](crate::SimpleTree)
    /// with default settings:
    /// - Number of leaves is unknown (will be counted during parsing)
    /// - Verbatim label resolution
    pub fn new_simple_defaults() -> Self {
        let storage = SimpleLabelStorage::default();
        Self {
            know_num_leaves: false,
            num_leaves: DEFAULT_NUM_LEAVES_GUESS,
            tree_builder: SimpleTreeBuilder::new(),
            resolver: LabelResolver::VerbatimLabels(storage),
        }
    }
}

// ============================================================================
// API Parsing (pub)
// ============================================================================
impl<T: TreeBuilder> NewickParser<T> {
    /// Consumes the parser and returns an iterator over trees from the byte source.
    ///
    /// The parser can be retrieved again via [NewickIterator::into_parser].
    ///
    /// # Arguments
    /// * `byte_parser` - A byte parser with underlying source containing only
    ///   Newick strings, except for whitespace and `[...]` comments.
    ///
    /// # Returns
    /// A [NewickIterator] allowing lazy parsing of trees.
    pub fn into_iter<B: ByteSource>(self, byte_parser: ByteParser<B>) -> NewickIterator<B, T> {
        NewickIterator {
            byte_parser,
            parser: self,
            done: false,
        }
    }

    /// Parses all Newick trees from the byte source until EOF.
    ///
    /// # Arguments
    /// * `byte_parser` - A byte parser with underlying source containing only
    ///   Newick strings, except for whitespace and `[...]` comments.
    ///
    /// # Returns
    /// * `Ok(Vec<T::Tree>)` - All parsed trees
    /// * `Err(ParsingError)` - If any tree fails to parse
    pub fn parse_all<B: ByteSource>(
        &mut self,
        mut byte_parser: ByteParser<B>,
    ) -> Result<Vec<T::Tree>, ParsingError> {
        let mut trees = Vec::new();
        loop {
            byte_parser.skip_comment_and_whitespace()?;
            if byte_parser.is_eof() {
                break;
            }
            trees.push(self.parse_str(&mut byte_parser)?);
        }
        Ok(trees)
    }

    /// Parses a single Newick tree from the given [ByteParser].
    ///
    /// # Arguments
    /// * `parser` - The byte parser positioned at the start of a Newick tree string
    ///
    /// # Returns
    /// * `Ok(T::Tree)` - The parsed phylogenetic tree
    /// * `Err(ParsingError)` - If the Newick format is invalid
    ///
    pub fn parse_str<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<T::Tree, ParsingError> {
        self.parse_str_and_name(parser, None)
    }

    /// Parses a single Newick tree from the given [ByteParser]
    /// and gives it the provided name.
    ///
    /// # Arguments
    /// * `parser` - The byte parser positioned at the start of a Newick tree string
    /// * `tree_name` - The name to give to the parsed tree
    ///
    /// # Returns
    /// * `Ok(T::Tree)` - The parsed phylogenetic tree
    /// * `Err(ParsingError)` - If the Newick format is invalid
    ///
    pub(crate) fn parse_str_and_name<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
        tree_name: Option<String>,
    ) -> Result<T::Tree, ParsingError> {
        self.tree_builder.init_next(self.num_leaves);

        if let Some(name) = tree_name {
            self.tree_builder.set_name(name);
        }

        // If number of leaves not know yet, reset it to 0,
        // so actual count can now be tracked
        if !self.know_num_leaves {
            self.num_leaves = 0;
        }

        self.parse_root(parser)?;

        // Having parsed a full tree,
        // the number of leaves in a tree is now known
        self.know_num_leaves = true;

        Ok(self.tree_builder.finish_tree().unwrap())
    }
}

// ============================================================================
// Parsing
// ============================================================================
impl<T: TreeBuilder> NewickParser<T> {
    /// Parses root of tree and adds it to tree:
    /// - `(left, right)[:branch_length]`
    /// - Skips leading comments and whitespace
    /// - Calls `parse_children` to parse the children pair
    ///
    /// Equivalent to `parse_internal_vertex` but takes care of root specialities.
    fn parse_root<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<(), ParsingError> {
        parser.skip_comment_and_whitespace()?;

        let (left_index, right_index) = self.parse_children(parser)?;

        // Root may have an optional branch length (might be None)
        let branch_length = self.parse_branch_length(parser)?;

        // Consume the terminating semicolon
        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b';') {
            let next_char = parser.peek().map(char::from);
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected ';' at end of tree but found {:?}", next_char),
            ));
        }

        self.tree_builder
            .add_root((left_index, right_index), branch_length);

        Ok(())
    }

    /// Parses a vertex (either internal vertex or leaf) and returns its vertex:
    /// - Skips leading comments and whitespace
    /// - Dispatches to `parse_internal_vertex` if starts with `(`, otherwise `parse_leaf`
    ///
    /// # Returns
    /// - vertex index of parsed internal vertex
    /// - [ParsingError] if something went wrong
    fn parse_vertex<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<T::VertexIdx, ParsingError> {
        parser.skip_comment_and_whitespace()?;
        if parser.peek_is(b'(') {
            self.parse_internal_vertex(parser)
        } else {
            self.parse_leaf(parser)
        }
    }

    /// Parses internal vertex, adds it to tree, and returns its index:
    /// - `(left, right)[:branch_length]`
    /// - Calls `parser_children` to parse the children pair
    ///
    /// # Returns
    /// - vertex index of parsed internal vertex
    /// - [ParsingError] if something went wrong
    fn parse_internal_vertex<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<T::VertexIdx, ParsingError> {
        let (left_index, right_index) = self.parse_children(parser)?;
        // Annotation parser will be added here.
        let branch_length = self.parse_branch_length(parser)?;
        let index = self
            .tree_builder
            .add_internal((left_index, right_index), branch_length);
        Ok(index)
    }

    /// Parses children pair `(left, right)` and returns their indices:
    /// - Expects parser at opening `(`
    ///   (caller should skip leading comments/whitespace)
    ///
    /// # Returns
    /// - vertex indices of left and right child vertices
    /// - [ParsingError] if something went wrong
    fn parse_children<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<(T::VertexIdx, T::VertexIdx), ParsingError> {
        // Parse: "(left"
        // Calling methods should have skipped comments and whitespace
        if !parser.consume_if(b'(') {
            let next_char = parser.peek().map(char::from);
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected '(' before children but found {:?}", next_char),
            ));
        }
        let left_index = self.parse_vertex(parser)?;

        // Parse: ",right"
        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b',') {
            let next_char = parser.peek().map(char::from);
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected ',' between children but found {:?}", next_char),
            ));
        }
        let right_index = self.parse_vertex(parser)?;

        // Parse: ")"
        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b')') {
            let next_char = parser.peek().map(char::from);
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected ')' after children but found {:?}", next_char),
            ));
        }

        Ok((left_index, right_index))
    }

    /// Parses leaf vertex and adds it to tree:
    /// - `label[:branch_length]`
    /// - Expects parser at start of label
    ///   (caller should skip leading comments/whitespace)
    ///
    /// # Returns
    /// - vertex index of parsed leaf
    /// - [ParsingError] if something went wrong,
    ///   e.g. if label couldn't be resolved
    fn parse_leaf<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<T::VertexIdx, ParsingError> {
        let label = parser.parse_label(NEWICK_LABEL_DELIMITERS)?;
        let label_ref = self
            .resolver
            .resolve_label(&label)
            .map_err(|e| ParsingError::unresolved_label(parser, e.to_string()))?;
        // Annotation parser will be added here.
        let branch_length = self.parse_branch_length(parser)?;
        if !self.know_num_leaves {
            self.num_leaves += 1;
        }

        Ok(self.tree_builder.add_leaf(branch_length, label_ref))
    }

    /// Parses optional branch length `[:number]`:
    /// - Skips comments/whitespace before and after `:`
    /// - Supports scientific notation (e.g., `1.5e-10`)
    ///
    /// # Returns
    /// -  `Ok(Some(branch_length))` if found a branch length and was able to parse it
    /// - `Ok(None)` if no branch length found
    /// - [ParsingError] if it couldn't parse branch length value
    fn parse_branch_length<B: ByteSource>(
        &mut self,
        parser: &mut ByteParser<B>,
    ) -> Result<Option<f64>, ParsingError> {
        // Parse: Whitespace/Comments : Whitespace/Comments
        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b':') {
            return Ok(None);
        }
        parser.skip_comment_and_whitespace()?;

        // Find end of branch length substring
        let mut branch_length_str = String::new();
        while let Some(b) = parser.peek() {
            // Valid characters for a float: digits, '.', '-', '+', 'e', 'E'
            if b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'+' || b == b'e' || b == b'E' {
                branch_length_str.push(b as char);
                parser.next_byte(); // consume it
            } else {
                break; // Hit a delimiter like ',', ')', ';', or whitespace
            }
        }

        // Parse branch length substring
        let value: f64 = branch_length_str.parse().map_err(|_| {
            ParsingError::invalid_newick_string(
                parser,
                format!("Invalid branch length: {}", branch_length_str),
            )
        })?;
        Ok(Some(value))
    }
}

// =#========================================================================#=
// NEWICK ITERATOR (lazy parser)
// =#========================================================================$=
/// Iterator to parse Newick trees.
///
/// Created by [NewickParser::into_iter()].
/// Yields `Result<T::Tree, ParsingError>` for each tree.
///
/// After iteration, the underlying [NewickParser] can be retrieved
/// via [into_parser()](Self::into_parser) to access the [TreeBuilder]
/// or other state.
pub struct NewickIterator<B, T>
where
    B: ByteSource,
    T: TreeBuilder,
{
    parser: NewickParser<T>,
    byte_parser: ByteParser<B>,
    done: bool,
}

impl<B, T> NewickIterator<B, T>
where
    B: ByteSource,
    T: TreeBuilder,
{
    /// Consumes the iterator and returns the underlying [NewickParser].
    pub fn into_parser(self) -> NewickParser<T> {
        self.parser
    }
}

impl<B, T> Iterator for NewickIterator<B, T>
where
    B: ByteSource,
    T: TreeBuilder,
{
    type Item = Result<T::Tree, ParsingError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        match self.parser.parse_str(&mut self.byte_parser) {
            Ok(tree) => {
                // Prepare for next call: skip whitespace and check EOF
                if let Err(e) = self.byte_parser.skip_comment_and_whitespace() {
                    self.done = true;
                    return Some(Err(e));
                }

                if self.byte_parser.is_eof() {
                    self.done = true;
                }

                Some(Ok(tree))
            }
            Err(err) => {
                self.done = true;
                Some(Err(err))
            }
        }
    }
}

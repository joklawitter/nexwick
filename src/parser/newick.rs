use crate::model::tree::{LabelIndex, LeafLabelMap, Tree, TreeIndex};
use crate::model::vertex::BranchLength;
use crate::parser::byte_parser::ByteParser;
use crate::parser::parsing_error::ParsingError;
use std::collections::HashMap;
use std::fmt;

/// Newick label delimiters: parentheses, comma, colon, semicolon, whitespace
const NEWICK_LABEL_DELIMITERS: &[u8] = b"([,:; \n\t\r)]";

/// Default guess for number of leaves, when unknown
const DEFAULT_NUM_LEAVES_GUESS: usize = 10;

/// Parser (configuration) for Newick format (binary) phylogenetic [Tree]s.
///
/// Supports parsing single or multiple Newick trees. Uses a [LabelResolver]
/// mechanism to turn ids or labels in Newick strings into a shared [LeafLabelMap].
/// A Nexus parser with a `TRANSLATE` command needs to provide the right [LabelResolver].
///
/// # Configuration
/// * `with_num_leaves(num_leaves)` - Can be configured with number of leaves in trees to parse,
///    otherwise it is inferred from the first parsed tree and then stored.
/// * `with_resolver(resolver)` - Requires a [LabelResolver] if labels are not stored directly in newick strings.
/// * Plan to include in the future `with_annotations()`, so that it can be configured
///   to parse vertex annotation instead of considering them comments,
///   (e.g. extract `pop_size` and value from "A[&pop_size=0.123]").
///   For now, annotation remains unsopported.
///
/// # Format
/// The Newick format has the following simple structure:
/// * tree ::= vertex ';'
/// * vertex ::= leaf | internal_vertex
/// * internal_vertex ::= '(' vertex ',' vertex ')' [branch_length]
/// * leaf ::= label [branch_length]
/// * branch_length ::= ':' number
///
/// Furthermore:
/// * Whitespace can occur between elements,
///   just not within unquoted label or in branch_length
/// * Even newlines can occur anywhere except in labels (quoted and unquoted)
/// * Comments are square brackets and can occur anywhere where newlines are allowed
///
/// In the extended Newick format, there can be comment-like annotation:
/// * `[@pop_size=0.543,color=blue]`
/// For a leaf:
/// * label [annotation] [branch_length]
///   - Example: A[@pop_size=0.543]:2.1
/// For an internal vertex and the root:
/// * (children) [annotation] [branch_length]
///   - Example: (A,B)[@pop_seize=0.345]:6.7
/// These are considered comments for now and skipped.
///
/// # Example
/// ```
/// use nexus_parser::parser::newick::NewickParser;
/// use nexus_parser::parser::byte_parser::ByteParser;
/// use nexus_parser::model::tree::{Tree, LeafLabelMap};
///
/// let input = "(A:1.0,B:1.0):0.0;";
/// let mut byte_parser = ByteParser::from_str(input);
///
/// let mut newick_parser = NewickParser::new().with_num_leaves(5); // create and configure
/// let tree = newick_parser.parse(&mut byte_parser).unwrap(); // let it parse to get tree
/// let labels = newick_parser.into_leaf_label_map(); // consume into LeafLabelMap
/// ```
pub struct NewickParser {
    know_num_leaves: bool,
    num_leaves: usize,
    resolver: LabelResolver,
    // parse_annotation: bool,
}

impl NewickParser {
    /// Creates a new `NewickParser` with default settings.
    ///
    /// By default,:
    /// - Number of leaves is unknown (will be counted during parsing),
    /// - Annotations are not parsed, and
    /// - No label resolver is set (will be created automatically if needed).
    pub fn new() -> Self {
        Self {
            know_num_leaves: false,
            num_leaves: DEFAULT_NUM_LEAVES_GUESS,
            resolver: LabelResolver::None,
            // parse_annotation: false,
        }
    }

    /// Sets the expected number of leaves in the tree.
    ///
    /// This allows pre-allocation of data structures for better performance.
    /// If not set, the parser will count leaves during parsing.
    pub fn with_num_leaves(mut self, num_leaves: usize) -> Self {
        self.num_leaves = num_leaves;
        self.know_num_leaves = true;
        self
    }

    // /// Enables parsing of tree annotations
    // pub fn with_annotations(mut self) -> Self {
    //     self.parse_annotation = true;
    //     self
    // }

    /// Sets a [LabelResolver] to resolve short/id keys or labels in Newick string
    /// to indices in [LeafLabelMap].
    pub fn with_resolver(mut self, resolver: LabelResolver) -> Self {
        self.resolver = resolver;
        self
    }

    /// Consumes the parser and returns the underlying [LeafLabelMap].
    ///
    /// This should be called after all trees have been parsed to retrieve
    /// the mapping of leaf labels to indices. This could either be a
    /// constructed [LeafLabelMap] or the originally provided via a [LabelResolver].
    pub fn into_leaf_label_map(self) -> LeafLabelMap {
        self.resolver.into_leaf_label_map()
    }

    /// Parses a single Newick tree from the given [ByteParser].
    ///
    /// The parser automatically creates a `LabelResolver` if none was provided.
    /// Subsequent calls to `parse()` will reuse the same resolver, allowing
    /// multiple trees to share the same [LeafLabelMap].
    ///
    /// # Arguments
    /// * `parser` - The byte parser positioned at the start of a Newick tree string
    ///
    /// # Returns
    /// * `Ok(Tree)` - The parsed phylogenetic tree
    /// * `Err(ParsingError)` - If the Newick format is invalid
    ///
    pub fn parse(&mut self, parser: &mut ByteParser) -> Result<Tree, ParsingError> {
        if let LabelResolver::None = self.resolver {
            let label_map = LeafLabelMap::new(self.num_leaves);
            self.resolver = LabelResolver::new_verbatim_labels_resolver(label_map);
        }

        let mut tree = Tree::new(self.num_leaves);

        // Reset number of leaves to 0, so we can now track it and determine the actual count
        if !self.know_num_leaves {
            self.num_leaves = 0;
        }

        self.parse_root(parser, &mut tree)?;

        // Having parsed a full tree, we now know the number of leaves in a tree
        self.know_num_leaves = true;

        Ok(tree)
    }

    /// Parses root of tree and adds it to tree:
    /// - `(left, right)[:branch_length]`
    /// - Skips leading comments and whitespace
    /// - Calls `parser_children` to parse the children pair
    ///
    /// Equivalent to `parse_internal_vertex` but taking care of root specialities
    fn parse_root(&mut self, parser: &mut ByteParser, tree: &mut Tree) -> Result<(), ParsingError> {
        parser.skip_comment_and_whitespace()?;

        let (left_index, right_index) = self.parser_children(parser, tree)?;

        // Root may have an optional branch length (which we ignore for now)
        if parser.peek() == Some(b':') {
            let _ = self.parse_branch_length(parser)?;
        }

        // Consume the terminating semicolon
        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b';') {
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected ';' at end of tree but found {:?}", parser.peek().map(|b| b as char)),
            ));
        }

        tree.add_root((left_index, right_index));

        Ok(())
    }

    /// Parses a vertex (either internal vertex or leaf) and returns its vertex:
    /// - Skips leading comments and whitespace
    /// - Dispatches to `parse_internal_vertex` if starts with `(`, otherwise `parse_leaf`
    ///
    /// # Returns
    /// - [TreeIndex] of parsed internal vertex
    /// - [ParsingError] if something went wrong
    fn parse_vertex(&mut self, parser: &mut ByteParser, tree: &mut Tree) -> Result<TreeIndex, ParsingError> {
        parser.skip_comment_and_whitespace()?;
        if parser.peek_is(b'(') {
            self.parse_internal_vertex(parser, tree)
        } else {
            self.parse_leaf(parser, tree)
        }
    }

    /// Parses internal vertex, adds it to tree, and returns its index:
    /// - `(left, right)[:branch_length]`
    /// - Calls `parser_children` to parse the children pair
    ///
    /// # Returns
    /// - [TreeIndex] of parsed internal vertex
    /// - [ParsingError] if something went wrong
    fn parse_internal_vertex(&mut self, parser: &mut ByteParser, tree: &mut Tree) -> Result<TreeIndex, ParsingError> {
        let (left_index, right_index) = self.parser_children(parser, tree)?;
        // Annotation parsing will be added here.
        let branch_length = self.parse_branch_length(parser)?;

        let index = tree.add_internal_vertex((left_index, right_index), branch_length);

        Ok(index)
    }

    /// Parses children pair `(left, right)` and returns their indices:
    /// - Expects parser at opening `(`
    ///   (caller should skip leading comments/whitespace)
    ///
    /// # Returns
    /// - [TreeIndex]s of left and right child vertices
    /// - [ParsingError] if something went wrong
    fn parser_children(&mut self, parser: &mut ByteParser, tree: &mut Tree) -> Result<(TreeIndex, TreeIndex), ParsingError> {
        // Calling methods should have skipped comments and whitespace
        if !parser.consume_if(b'(') {
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected '(' before children but found {:?}", parser.peek().map(|b| b as char)),
            ));
        }
        let left_index = self.parse_vertex(parser, tree)?;

        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b',') {
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected ',' between children but found {:?}", parser.peek().map(|b| b as char)),
            ));
        }
        let right_index = self.parse_vertex(parser, tree)?;

        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b')') {
            return Err(ParsingError::invalid_newick_string(
                parser,
                format!("Expected ')' after children but found {:?}", parser.peek().map(|b| b as char)),
            ));
        }

        Ok((left_index, right_index))
    }

    /// Parses leaf vertex and adds it to tree:
    /// - `label[:branch_length]`
    /// - Expects parser at start of label
    ///   (caller should skip leading comments/whitespace)
    /// - Resolves label via the configured resolver
    ///
    /// # Returns
    /// - [TreeIndex] of parsed leaf
    /// - [ParsingError] if something went wrong
    fn parse_leaf(&mut self, parser: &mut ByteParser, tree: &mut Tree) -> Result<TreeIndex, ParsingError> {
        let label = parser.parse_label(NEWICK_LABEL_DELIMITERS)?;
        // Annotation parsing will be added here.
        let label_index = self.resolver.resolve_label(&*label, parser)?;
        let branch_length = self.parse_branch_length(parser)?;

        let index = tree.add_leaf(branch_length, label_index);
        if !self.know_num_leaves {
            self.num_leaves += 1;
        }

        Ok(index)
    }

    /// Parses optional branch length `[:number]`:
    /// - Skips comments/whitespace before and after `:`
    /// - Supports scientific notation (e.g., `1.5e-10`)
    ///
    /// # Returns
    /// - [BranchLength] if found branch length and was able to parse it
    /// - `None` if found no branch length
    /// - [ParsingError] if it couldn't parse branch length value
    fn parse_branch_length(&mut self, parser: &mut ByteParser) -> Result<Option<BranchLength>, ParsingError> {
        // Whitespace/Comments : Whitespace/Comments
        parser.skip_comment_and_whitespace()?;
        if !parser.consume_if(b':') {
            return Ok(None);
        }
        parser.skip_comment_and_whitespace()?;

        let mut branch_length_str = String::new();
        while let Some(b) = parser.peek() {
            // Valid characters for a float: digits, '.', '-', '+', 'e', 'E'
            if b.is_ascii_digit() || b == b'.' || b == b'-' || b == b'+' || b == b'e' || b == b'E' {
                branch_length_str.push(b as char);
                parser.next(); // consume it
            } else {
                break; // Hit a delimiter like ',', ')', ';', or whitespace
            }
        }

        let value: f64 = branch_length_str.parse()
            .map_err(|_| ParsingError::invalid_newick_string(parser, format!("Invalid branch length: {}", branch_length_str)))?;
        Ok(Some(BranchLength::new(value)))
    }
}

/// Resolves leaf labels to indices during Newick tree parsing,
/// using or building a [LeafLabelMap].
///
/// This enum handles different scenarios for mapping labels in Newick trees:
/// - Verbatim label parsing (raw Newick strings)
/// - Translation-based parsing (NEXUS TRANSLATE blocks)
///
/// # Comment
/// Since in practice often actually uses TRANSLATE block with indices,
/// could add a faster array based resolver.
#[derive(Debug)]
pub enum LabelResolver {
    /// Resolves direct verbatim label -> index mapping for raw Newick strings.
    ///
    /// # Warning
    /// Even a NEXUS file without a TRANSLATE command may use id keys
    /// based on the order in which the labels were defined in the TAXA block.
    ///
    /// Example: "White-fronted tern" → index 11
    VerbatimLabels(LeafLabelMap),

    /// Resolves all allowed types of keys and labels in Newick strings of Nexus TREES command,
    /// using mapping from TRANSLATE block, if provided:
    /// - Key provided by mapping
    /// - Otherwise and if integer key, the index (1-based indexing) in label definitions of TAXA block
    /// - Otherwise, verbatim label also still allowed
    ///
    /// # Examples
    /// - "terny" -> ("White-fronted tern" ->) index 11
    /// - 12 (1-based index) (-> "White-fronted tern" ->) index 11
    /// - "White-fronted tern" -> index 11
    NexusLabels {
        /// Pre-computed mapping from keys to leaf indices
        index_map: HashMap<String, LabelIndex>,
        /// The complete mapping of labels to indices
        leaf_label_map: LeafLabelMap,
    },

    /// Resolves only integer keys in Newick strings of Nexus TREES command,
    /// using mapping from TRANSLATE block, necessarily provided.
    ///
    /// Optimizes for TRANSLATE blocks using consecutive integer keys (1, 2, 3, ...)
    /// thus allow direct array lookup: translate_index -> leaf_label_index
    NexusIntegerLabels {
        index_array: Vec<LabelIndex>,  // translate_index[i] = leaf_label_map index
        leaf_label_map: LeafLabelMap,
    },

    /// Uninitialized resolver (will be replaced when parsing starts)
    None,
}

impl LabelResolver {
    /// Creates a `VerbatimLabels` resolver for verbatim label parsing.
    ///
    /// Use this when parsing:
    /// - Raw Newick files without translation tables
    /// - NEXUS files without TRANSLATE blocks
    ///
    /// # Arguments
    /// * `leaf_map` - An existing or new `LeafLabelMap` to populate
    pub fn new_verbatim_labels_resolver(leaf_map: LeafLabelMap) -> Self {
        LabelResolver::VerbatimLabels(leaf_map)
    }

    /// Creates a `NexusLabels` resolver for NEXUS file Newick tree parsing.
    ///
    /// # Arguments
    /// * `translation` - Map from short keys/IDs to full taxon labels
    /// * `leaf_label_map` - [LeafLabelMap] based on TAXA block (including ordering)
    ///
    /// # Panics
    /// Panics if a label provided by `translation` does not appear in the provided [LeafLabelMap].
    pub fn new_nexus_labels_resolver(translation: HashMap<String, String>, leaf_label_map: LeafLabelMap) -> Self {
        // Instead of going from key -> label and then from label -> index,
        // we create a direct mapping
        let mut index_map = HashMap::with_capacity(translation.len());
        for (key, actual_label) in &translation {
            let label_index = leaf_label_map.get_index(actual_label)
                .expect(format!("Label {} provided by translation should have been present in provided LeafLabelMap.", actual_label).as_str());
            index_map.insert(key.clone(), label_index);
        }

        LabelResolver::NexusLabels { index_map, leaf_label_map }
    }

    /// Creates a `NexusIntegerLabels` resolver for NEXUS file Newick tree parsing.
    ///
    /// # Arguments
    /// * `translation` - Map from integer keys (1, 2, 3, ...) to full taxon labels
    /// * `leaf_label_map` - [LeafLabelMap] based on TAXA block (including ordering)
    ///
    /// # Panics
    /// Panics if:
    /// - A key is not a valid positive integer
    /// - A key is out of bounds (0 or > num_labels)
    /// - A label provided by `translation` does not appear in the provided [LeafLabelMap];
    ///     you can check consistent with `leaf_label_map.check_consistency(translation)` beforehand
    /// - Keys are not consecutive integers starting from 1
    pub fn new_nexus_integer_labels_resolver(translation: HashMap<String, String>, leaf_label_map: LeafLabelMap) -> Self {
        let num_labels = leaf_label_map.num_labels();

        // Validate all keys are valid integers and build index array;
        // Array at position i contains the label index for NEXUS index i (1-based, so NEXUS "1" is at index_array[0])
        let mut index_array = vec![0; num_labels];

        for (key, actual_label) in &translation {
            // Parse key as integer
            let nexus_index = key.parse::<usize>()
                .expect(&format!("TRANSLATE key '{}' is not a valid integer", key));

            // Validate bounds (1-based NEXUS indexing)
            if nexus_index == 0 || nexus_index > num_labels {
                panic!("TRANSLATE index {} out of bounds (1-based indexing, valid range: 1-{})",
                       nexus_index, num_labels);
            }

            // Look up the label in the leaf_label_map
            let label_index = leaf_label_map.get_index(actual_label)
                .expect(&format!("Label '{}' provided by translation not found in LeafLabelMap", actual_label));

            // Store in array (converting from 1-based to 0-based indexing)
            index_array[nexus_index - 1] = label_index;
        }

        LabelResolver::NexusIntegerLabels { index_array, leaf_label_map }
    }

    /// Resolves a parsed label string to its corresponding `LabelIndex`.
    ///
    /// # Arguments
    /// * `parsed_label` - The label string extracted from the Newick tree
    /// * `parser` - The byte parser (used for error reporting)
    ///
    /// # Returns
    /// * `Ok(LabelIndex)` - The index corresponding to this label
    /// * `Err(ParsingError)` - If the label cannot be resolved
    pub fn resolve_label(&mut self, parsed_label: &str, parser: &ByteParser) -> Result<LabelIndex, ParsingError> {
        match self {
            LabelResolver::VerbatimLabels(leaf_label_map) => {
                Ok(leaf_label_map.get_or_insert(parsed_label))
            }

            LabelResolver::NexusLabels { index_map, leaf_label_map } => {
                // 1. Try if parsed label is key of translation map
                let index = index_map.get(parsed_label);
                if let Some(index) = index {
                    return Ok(*index);
                }

                // 2. Try if parsed label is integer
                let nexus_index = parsed_label.parse::<usize>();
                if let Ok(nexus_index) = nexus_index {
                    if nexus_index == 0 || nexus_index > leaf_label_map.num_labels() {
                        return Err(ParsingError::unresolved_label(
                            parser,
                            format!("Nexus label index {nexus_index} out of bounds (1-based indexing, max {})", leaf_label_map.num_labels()),
                        ));
                    }
                    return Ok(nexus_index - 1);
                }

                // 3. Try if parsed label is verbatim label
                let verbatim_try = leaf_label_map.get_index(parsed_label);
                if let Some(verbatim_try) = verbatim_try {
                    return Ok(verbatim_try);
                }


                Err(ParsingError::unresolved_label(parser, format!("NexusResolver could not resolve {parsed_label}")))
            }

            LabelResolver::NexusIntegerLabels { index_array, .. } => {
                // Try if parsed label is integer (1-based index)
                if let Ok(nexus_index) = parsed_label.parse::<usize>() {
                    // Validate bounds (1-based NEXUS indexing)
                    if nexus_index == 0 || nexus_index > index_array.len() {
                        return Err(ParsingError::unresolved_label(
                            parser,
                            format!("Index {} out of bounds (1-based indexing, valid range: 1-{})",
                                    nexus_index, index_array.len()),
                        ));
                    }
                    // Convert 1-based to 0-based and lookup in array
                    return Ok(index_array[nexus_index - 1]);
                }

                Err(ParsingError::unresolved_label(
                    parser,
                    format!("NexusIntegerLabels resolver requires integer labels, got '{}'", parsed_label),
                ))
            }

            LabelResolver::None => {
                Err(ParsingError::unresolved_label(parser, "No resolver initialized".to_string()))
            }
        }
    }

    /// Consumes the resolver and returns the stored or accumulated `LeafLabelMap`.
    ///
    /// This extracts the final label-to-index mapping from the resolver,
    /// regardless of which variant it is.
    ///
    /// # Returns
    /// The [LeafLabelMap] initially supplied or a new one containing all labels encountered during parsing.
    /// Returns an empty map if the resolver was never initialized (`None` variant).
    pub fn into_leaf_label_map(self) -> LeafLabelMap {
        match self {
            LabelResolver::VerbatimLabels(leaf_label_map) => leaf_label_map,
            LabelResolver::NexusLabels { leaf_label_map, .. } => leaf_label_map,
            LabelResolver::NexusIntegerLabels { leaf_label_map, .. } => leaf_label_map,
            LabelResolver::None => LeafLabelMap::new(0),
        }
    }
}

impl fmt::Display for LabelResolver {
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
                for (i, &label_index) in index_array.iter().enumerate() {
                    writeln!(f, "  {} -> {}", i + 1, label_index)?;
                }
                Ok(())
            }
            LabelResolver::None => {
                writeln!(f, "LabelResolver::None")
            }
        }
    }
}
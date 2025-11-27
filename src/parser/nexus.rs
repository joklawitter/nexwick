use crate::model::tree::{LeafLabelMap, Tree};
use crate::parser::byte_parser::ByteParser;
use crate::parser::byte_parser::ConsumeMode::{Exclusive, Inclusive};
use crate::parser::newick::{LabelResolver, NewickParser};
use crate::parser::parsing_error::ParsingError;
use std::collections::HashMap;

// Nexus label delimiters: parentheses, comma, colon, semicolon, whitespace
const NEXUS_LABEL_DELIMITERS: &[u8] = b" ,;\t\n\r";

#[derive(Debug, PartialEq, Clone)]
pub enum NexusBlock {
    Taxa,
    Trees,
    Data,
    Characters,
    Distances,
    Sets,
    Assumptions,
    UnknownBlock(String),
    Other,
}

impl NexusBlock {
    /// Parse a block name (case-insensitive) into a NexusBlock variant
    pub fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "taxa" => NexusBlock::Taxa,
            "trees" => NexusBlock::Trees,
            "data" => NexusBlock::Data,
            "characters" => NexusBlock::Characters,
            "distances" => NexusBlock::Distances,
            "sets" => NexusBlock::Sets,
            "assumptions" => NexusBlock::Assumptions,
            _ => NexusBlock::UnknownBlock(name.to_string()),
        }
    }
}

pub fn parse_nexus(parser: &mut ByteParser) -> Result<(Vec<Tree>, LeafLabelMap), ParsingError> {
    // println!("Parsing nexus file.");

    // Parse and verify header
    parse_nexus_header(parser)?;
    // println!("Parsing header successful.");

    // Skip until TAXA block and parse it
    // println!("Parsing taxa block...");
    skip_until_block(parser, NexusBlock::Taxa)?;
    let (num_taxa, label_map) = parse_taxa_block(parser)?;
    // println!("... successful.");
    // println!("Num taxa: {}", num_taxa);
    // println!("{}", label_map);

    // Skip until TREES block and parse it
    // println!("Parsing trees block...");
    skip_until_block(parser, NexusBlock::Trees)?;
    let (trees, label_map) = parse_tree_block(parser, num_taxa, label_map)?;
    // println!("... successful.");

    Ok((trees, label_map))
}

/// Parse header `#NEXUS` at start of file or throw `ParsingError::MissingNexusHeader` otherwise.
fn parse_nexus_header(parser: &mut ByteParser) -> Result<(), ParsingError> {
    parser.skip_comment_and_whitespace()?;

    if !parser.consume_if_word("#NEXUS") {
        return Err(ParsingError::missing_nexus_header(parser));
    }

    Ok(())
}

/// Skip Nexus blocks until we encounter the target block type; returns `[ParsingError::UnexpectedEOF]` if not found.
fn skip_until_block(parser: &mut ByteParser, target: NexusBlock) -> Result<(), ParsingError> {
    loop {
        if parser.is_eof() {
            return Err(ParsingError::unexpected_eof(parser));
        }

        let block_type = detect_next_block(parser)?;

        if block_type == target {
            return Ok(());
        }

        skip_to_block_end(parser)?;
    }
}

/// Detect the next Nexus block, which must start with `BEGIN <BlockType>;` (case-insensitive),
/// and return its BlockType, or a ParsingError if something wrong.
fn detect_next_block(parser: &mut ByteParser) -> Result<NexusBlock, ParsingError> {
    parser.skip_comment_and_whitespace()?;

    if !parser.consume_if_word("BEGIN") {
        return Err(ParsingError::invalid_formatting(parser));
    }
    parser.skip_comment_and_whitespace()?;

    let start_pos = parser.position();
    if !parser.consume_until(b';', Exclusive) {
        return Err(ParsingError::unexpected_eof(parser));
    }

    let block_name = std::str::from_utf8(parser.slice_from(start_pos))
        .map_err(|_| ParsingError::invalid_block_name(parser))?
        .to_string();

    parser.next(); // consume the ';' now (already know that this is next byte)

    Ok(NexusBlock::from_name(&block_name))
}

/// Skip block, continuing until encountered `END;`.
fn skip_to_block_end(parser: &mut ByteParser) -> Result<(), ParsingError> {
    if !parser.consume_until_sequence(b"End;", Inclusive) {
        return Err(ParsingError::unexpected_eof(parser));
    }

    Ok(())
}

/// Parse TAXA block extracting number of taxa from `ntax` command and taxon list from `TAXLABEL` command,
/// ignoring any other command and comments.
///
/// # Assumptions
/// * First command must be `DIMENSIONS NTAX=<value>;` (case-insensitive)
///   - `<value>` must be integer and semicolon
///   - No comment within command allowed
/// * Followed by list of labels command  `TAXLABEL [label1 label2 ...];`, with following details:
///   - Space separated list of labels
///   - Terminated by semicolon
///   - Comments allowed
/// * Comments allowed outside the two commands
///
/// # Errors
/// Return [ParsingError::UnexpectedEOF] if block, command, or comment not properly closed,
/// and [ParsingErrror::InvalidTaxaBlock] if command tokes not encountered in expected order (as specified above).
fn parse_taxa_block(parser: &mut ByteParser) -> Result<(usize, LeafLabelMap), ParsingError> {
    // 1. Parse number of taxa command "DIMENSIONS NTAX=n;"
    let num_taxa = parse_taxa_block_ntax(parser)?;

    // 2. Parse list of taxa labels in `TAXLABEL` command, including consuming closing ";"
    let label_map = parse_taxa_block_labels(parser, num_taxa)?;

    // 3. Move to end of block
    skip_to_block_end(parser)?;
    // Would expect only "END;" besides whitespace and comments, but not enforced here

    Ok((num_taxa, label_map))
}

/// Helper method to parse TAXA block, responsible for parsing `ntax` command
/// and returning the result: the number of taxa.
fn parse_taxa_block_ntax(parser: &mut ByteParser) -> Result<usize, ParsingError> {
    // This could be a replaced with a general parsing structure for such a type of command,
    // but for our nexus files we only have this one.
    // a) Parse "DIMENSIONS NTAX="
    parser.skip_comment_and_whitespace()?;
    if !parser.consume_if_word("DIMENSIONS") {
        return Err(ParsingError::invalid_taxa_block(parser, String::from("Expected 'DIMENSIONS' in TAXA block.")));
    }

    parser.skip_whitespace();
    if !parser.consume_if_word("NTAX") {
        return Err(ParsingError::invalid_taxa_block(parser, String::from("Expected 'NTAX' in TAXA block.")));
    }

    parser.skip_whitespace();
    if !parser.consume_if(b'=') {
        return Err(ParsingError::invalid_taxa_block(parser, String::from("Expected '=' in TAXA block.")));
    }

    // b) Read the number `n` and consume ";"
    parser.skip_whitespace();
    let start_pos = parser.position();
    if !parser.consume_until(b';', Exclusive) {
        return Err(ParsingError::unexpected_eof(parser));
    }
    let ntax_str = std::str::from_utf8(parser.slice_from(start_pos))
        .map_err(|_| ParsingError::invalid_taxa_block(parser, String::from("Invalid UTF-8 in ntax value")))?
        .trim();
    let ntax: usize = ntax_str.parse()
        .map_err(|_| ParsingError::invalid_taxa_block(parser, format!("Cannot parse `ntax` value: {}", ntax_str)))?;
    parser.next(); // consume the semicolon

    Ok(ntax)
}

/// Helper method to parse TAXA block, responsible for parsing `TAXLABEL` command
/// and returning the parsed taxa as [LeafLabelMap].
fn parse_taxa_block_labels(parser: &mut ByteParser, num_taxa: usize) -> Result<LeafLabelMap, ParsingError> {
    // a) Parse "TAXLABELS"
    parser.skip_comment_and_whitespace()?;
    if !parser.consume_if_sequence(b"TAXLABELS") {
        return Err(ParsingError::invalid_taxa_block(parser, String::from("Expected 'TAXLABELS' in TAXA block.")));
    }

    // b) Read labels until semicolon
    let mut label_map = LeafLabelMap::new(num_taxa);
    loop {
        parser.skip_comment_and_whitespace()?;

        // Stop once encountering semicolon (end of labels command)
        if parser.peek() == Some(b';') {
            parser.next();
            break;
        }

        // Read one label (word until whitespace or semicolon)
        let label = parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

        if !label.is_empty() {
            label_map.insert(label);
        }
    }

    // c) Check that `num_taxa` many labels parsed
    if !label_map.is_full() {
        return Err(ParsingError::invalid_taxa_block(
            parser,
            format!("Number of parsed labels ({}) did not match ntax value ({}).",
                    label_map.num_labels(), num_taxa)));
    }

    Ok(label_map)
}

/// Parse TREES block extracting all trees non-lazily.
/// Resolves taxon labels based on provided mapping ("TRANSLATE" command)
/// or by implicit label order (1-indexed) as specified in TAXA blog.
///
/// # Assumptions
/// * A "TRANSLATE" command, if present, must precede any "TREE" command, with following details:
///   - Command is a comma seperated list of pairs of "id/short label":
///         `TRANSLATE [<key1=short1/id1> <label1>, ...];`
///   - Mapping should *consistently* use ids (integer) or shorts as key; behaviour undefined otherwise
///   - `<label>` must match a label provided in TAXA blog.
///   - Length of mapping must match number of taxa/labels.
///         (This is a program specific requirement, not of NEXUS files.)
///   - A label with a space in it must be enclosed in single quotes and ...
///   - A label with an apostrophe in it must be enclosed in single quotes
///     and the apostrophe must be escaped with an apostrophe/single quote:
///     e.g. "Wilson's Storm-petrel" becomes 'Wilson''s_storm-petrel'
///   - No comments within pair allowed, only between comma and next pair,
///     e.g. "[cool seabird] stormy 'Wilson''s_storm-petrel',"
/// * Trees come in semicolon separated list of tree commands
/// * One tree command has format "tree <name> = <Newick string>;"
///   - Each pair is separated by a comma, optional whitespace and comments
///   - Only one mapping per taxon allowed, either
///   - Neither full nor short labels contain whitespace
fn parse_tree_block(parser: &mut ByteParser, num_leaves: usize, leaf_label_map: LeafLabelMap)
                    -> Result<(Vec<Tree>, LeafLabelMap), ParsingError> {
    let mut trees: Vec<Tree> = Vec::new();

    // 1. Try to parse TRANSLATE command
    let map = parse_tree_block_translate(parser, num_leaves)?;


    // ... and based on whether it exists, pick the appropriate label resolver
    let resolver = match map {
        None => {
            LabelResolver::new_verbatim_labels_resolver(leaf_label_map)
        }
        Some(map) => {
            // Assert that labels match those provided in TAXA block
            if !leaf_label_map.check_consistency_with_translation(&map) {
                return Err(ParsingError::invalid_translate_command(parser));
            }

            // Check if all keys are integers to use the more efficient NexusIntegerLabels resolver
            let all_keys_are_integers = map.keys().all(|key| key.parse::<usize>().is_ok());

            if all_keys_are_integers {
                LabelResolver::new_nexus_integer_labels_resolver(map, leaf_label_map)
            } else {
                LabelResolver::new_nexus_labels_resolver(map, leaf_label_map)
            }
        }
    };

    // 2. Parse trees
    let leaf_label_map = parse_tree_block_trees(parser, &mut trees, num_leaves, resolver)?;

    // 3. Consume rest including of block including "END;"
    skip_to_block_end(parser)?;


    Ok((trees, leaf_label_map))
}

/// Helper method to parse TREES block, responsible for parsing `TRANSLATE` command.
/// If command exists, returns parsed mapping.
fn parse_tree_block_translate(parser: &mut ByteParser, num_leaves: usize) -> Result<Option<HashMap<String, String>>, ParsingError> {
    // a) Parse "TRANSLATE"
    parser.skip_comment_and_whitespace()?;
    if !parser.consume_if_sequence(b"TRANSLATE") {
        // there might be no TRANSLATE command, which is fine if the next command is a TREE
        return if parser.peek_is_sequence(b"TREE") {
            Ok(None)
        } else {
            Err(ParsingError::invalid_taxa_block(parser, String::from("Expected 'TRANSLATE' or first 'TREE' in TREES block.")))
        };
    }

    // b) Parse pairs "id/short label"
    let mut map: HashMap<String, String> = HashMap::with_capacity(num_leaves);
    loop {
        parser.skip_comment_and_whitespace()?;

        // Read key (short label or id)
        let key = parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

        // Expect a space
        if !parser.consume_if(b' ') {
            return Err(ParsingError::invalid_trees_block(parser, String::from("Expected ' ' in between key and label.")));
        }

        // Parse label
        let label = parser.parse_label(NEXUS_LABEL_DELIMITERS)?;
        parser.skip_whitespace();

        // d) Add to HashMap
        map.insert(key, label);

        // e) Continue if next is a comma
        if parser.consume_if(b',') {
            continue;
        }
        // but stop if semicolon (end of "TRANSLATE" command)
        if parser.consume_if(b';') {
            break;
        }
        // and otherwise invalid
        let char = parser.peek().unwrap().to_string();
        return Err(ParsingError::invalid_trees_block(parser, format!("Unexpected char '{char}' in TRANSLATE.")));
    }


    // c) small check
    assert_eq!(map.len(), num_leaves);

    Ok(Option::from(map))
}

/// Helper method to parse TREES block, responsible for parsing 'TREE' entries.
/// Returns all trees parsed with labels resolved with given resolver.
fn parse_tree_block_trees(parser: &mut ByteParser, trees: &mut Vec<Tree>, num_leaves: usize, resolver: LabelResolver)
                          -> Result<LeafLabelMap, ParsingError> {
    let mut newick_parser = NewickParser::new().with_num_leaves(num_leaves).with_resolver(resolver);

    // let mut tree_count = 0;
    loop {
        parser.skip_comment_and_whitespace()?;

        // Stop if "END;"
        if parser.peek_is_word("END;") {
            break;
        }

        // Expect "TREE"
        if !parser.consume_if_sequence(b"tree") {
            return Err(ParsingError::invalid_trees_block(parser, String::from("Expected 'tree' in tree block.")));
        }

        // Parse tree name
        let name = parser.parse_label(NEXUS_LABEL_DELIMITERS)?;

        // Expect "="
        parser.skip_whitespace();
        if !parser.consume_if(b'=') {
            return Err(ParsingError::invalid_trees_block(parser, String::from("Expected '=' in tree block. ")));
        }

        // Skip optional "[&R/U]" (rooter or unrooted tree) by considering it a comment
        parser.skip_comment_and_whitespace()?;

        // Parse tree
        let tree = newick_parser.parse(parser)?.with_name(name);

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

    Ok(newick_parser.into_leaf_label_map())
}




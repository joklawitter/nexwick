use nexwick::parser::byte_parser::ConsumeMode::{Exclusive, Inclusive};
use nexwick::parser::byte_parser::{ByteParser, ConsumeMode};

#[test]
fn test_skip_whitespace() {
    let mut parser = ByteParser::for_str(" \r  \t\n \t x y");
    parser.skip_whitespace();
    assert_eq!(parser.peek(), Some(b'x'));

    parser.next_byte(); // skip x
    parser.skip_whitespace();
    assert_eq!(parser.peek(), Some(b'y'));
}

#[test]
fn test_skip_comment() {
    let mut parser = ByteParser::for_str("Tree tiny = [Following tree is tiny] ((A:1,B:1):1,C:2)");
    parser.consume_until(b'=', ConsumeMode::Inclusive);
    parser.skip_whitespace();
    assert!(parser.skip_comment().unwrap());
    assert_eq!(parser.next_byte(), Some(b' '));
    assert_eq!(parser.next_byte(), Some(b'('));
    assert!(!parser.skip_comment().unwrap());
}

#[test]
fn test_skip_comment_and_whitespace() {
    let mut parser =
        ByteParser::for_str("[Go] \n[Keep going]   \t ['...']\n[One more to go]  END!");
    parser
        .skip_comment_and_whitespace()
        .expect("Failed to skip comments.");
    assert_eq!(parser.next_byte(), Some(b'E'));
}

#[test]
fn test_consume_until_inclusive() {
    let mut parser = ByteParser::for_str("consume a CAN of beans");
    parser.consume_until(b'C', Inclusive);
    assert_eq!(parser.peek(), Some(b'A'));
    assert_eq!(parser.position(), 11);
}

#[test]
fn test_consume_until_exclusive() {
    let mut parser = ByteParser::for_str("consume a CAN of beans");
    parser.consume_until(b'C', Exclusive);
    assert_eq!(parser.peek(), Some(b'C'));
    assert_eq!(parser.position(), 10);
}

#[test]
fn test_consume_until_any_inclusive() {
    let mut parser = ByteParser::for_str("yummy! eat Apples\n");
    let targets = [b'B', b'A', b'n', b'a', b'\n', b'@'];
    let found = parser.consume_until_any(&targets, Inclusive);
    assert_eq!(found, Some(b'a'));
    assert_eq!(parser.position(), 9);
    let found = parser.consume_until_any(&targets, Inclusive);
    assert_eq!(found, Some(b'A'));
    assert_eq!(parser.position(), 12);
    let found = parser.consume_until_any(&targets, Inclusive);
    assert_eq!(found, Some(b'\n'));
    assert_eq!(parser.position(), 18);

    let mut parser = ByteParser::for_str("leek soup, hmm");
    let found = parser.consume_until_any(&targets, Inclusive);
    assert!(found.is_none());
}

#[test]
fn test_consume_until_any_exclusive() {
    let mut parser = ByteParser::for_str("find yet do not eat the Banana!");
    let targets = [b'b', b'B', b'q', b'!'];
    let found = parser.consume_until_any(&targets, Exclusive);
    assert_eq!(found, Some(b'B'));
    assert_eq!(parser.peek(), found);
    assert_eq!(parser.position(), 24);
}

#[test]
fn test_is_eof() {
    let mut parser = ByteParser::for_str("... happily ever after!");
    parser.consume_until(b'!', Inclusive);
    assert!(parser.is_eof());
}

#[test]
fn test_position() {
    let mut parser = ByteParser::for_str("Where are we?");
    assert_eq!(parser.position(), 0);
    parser.peek();
    assert_eq!(parser.position(), 0);
    parser.next_byte();
    assert_eq!(parser.position(), 1);
}

#[test]
fn test_peek_is_word() {
    let mut parser = ByteParser::for_str("BEGIN TREES;");
    assert!(parser.peek_is_word("BEGIN"));
    assert!(parser.peek_is_word("beGin"));
    assert!(!parser.peek_is_word("benin"));
    // Position should not have changed (peek operation)
    assert_eq!(parser.position(), 0);
    assert_eq!(parser.peek(), Some(b'B'));
}

#[test]
fn test_parse_unquoted_label() {
    let mut parser = ByteParser::for_str("Scarabaeus:0.5");
    let delimiters = b"(),:; \t\n\r";
    let label = parser.parse_unquoted_label(delimiters).unwrap();
    assert_eq!(label, "Scarabaeus");
    assert_eq!(parser.peek(), Some(b':'));
}

#[test]
fn test_parse_quoted_label() {
    let mut parser = ByteParser::for_str("'Scarabaeus viettei':0.5");
    let label = parser.parse_quoted_label().unwrap();
    assert_eq!(label, "Scarabaeus viettei");
    assert_eq!(parser.peek(), Some(b':'));
}

#[test]
fn test_parse_quoted_label_with_escaped_quote() {
    let mut parser = ByteParser::for_str("'Wilson''s_storm-petrel',");
    let label = parser.parse_quoted_label().unwrap();
    assert_eq!(label, "Wilson's_storm-petrel");
    assert_eq!(parser.peek(), Some(b','));
}

#[test]
fn test_parse_label_chooses_quoted() {
    let mut parser = ByteParser::for_str(" 'Quoted label' ");
    let delimiters = b"(),:; \t\n\r";
    let label = parser.parse_label(delimiters).unwrap();
    assert_eq!(label, "Quoted label");
}

#[test]
fn test_parse_label_chooses_unquoted() {
    let mut parser = ByteParser::for_str("  UnquotedLabel:");
    let delimiters = b"(),:; \t\n\r";
    let label = parser.parse_label(delimiters).unwrap();
    assert_eq!(label, "UnquotedLabel");
    assert_eq!(parser.peek(), Some(b':'));
}

#[test]
fn test_get_context() {
    let mut parser = ByteParser::for_str("Hello World!");
    assert_eq!(parser.get_context(5), b"Hello");

    parser.consume_if_word("Hello");
    parser.skip_whitespace();
    assert_eq!(parser.get_context(5), b"World");

    parser.consume_until_word("World", Inclusive);
    assert_eq!(parser.get_context(10), b"!");
}

#[test]
fn test_get_context_as_string() {
    let mut parser = ByteParser::for_str("BEGIN TREES;");
    assert_eq!(parser.get_context_as_string(5), "BEGIN");

    parser.consume_if_word("BEGIN");
    parser.skip_whitespace();
    assert_eq!(parser.get_context_as_string(10), "TREES;");
}

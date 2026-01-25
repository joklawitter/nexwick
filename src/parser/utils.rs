//! Utility functions for label escaping and unescaping in phylogenetic tree formats.
//!
//! This module provides functions for safely handling labels in NEXUS and Newick formats,
//! ensuring special characters are properly escaped (according to specifications)
//! when writing and unescaped when reading.

/// Checks if a label is already escaped:
/// - wrapped in single quotes and each internal single quote doubled, or
/// - no space and special characters
///
/// # Arguments
/// * `label` - The label string to check
///
/// # Returns
/// `true` if the label is properly escaped, `false` otherwise
///
/// # Examples
/// ```
/// # use nexwick::parser::utils::is_escaped;
/// assert_eq!(is_escaped("Pukeko"), true); // Also known as Australasian Swamphen
/// assert_eq!(is_escaped("Pu[ke]ko"), false);
/// assert_eq!(is_escaped("Australasian Swamphen"), false);
/// assert_eq!(is_escaped("Australasian_Swamphen"), true);
/// assert_eq!(is_escaped("'Australasian Swamphen'"), true);
/// assert_eq!(is_escaped("'Baillon''s_Crake'"), true); // Also known as Marsh Crake
/// assert_eq!(is_escaped("'Baillon's Crake'"), false); // Single quoted but unescaped internal single quote
/// ```
pub fn is_escaped(label: &str) -> bool {
    if is_single_quoted(label) {
        // Check that every internal single quote is escaped
        let inner = &label[1..label.len() - 1];
        let mut prev = ' ';
        for char in inner.chars() {
            if prev == '\'' {
                // Have to check if previous ' is followed by a '
                if char != '\'' {
                    return false;
                } else {
                    // Since we now have a pair of single quotes, set prev to something fine
                    prev = ' ';
                }
            } else {
                prev = char;
            }
        }

        true
    } else {
        !label.chars().any(|c| {
            matches!(
                c,
                ' ' | ',' | ';' | '\t' | '\n' | '\r' | '(' | ')' | ':' | '[' | ']' | '\''
            )
        })
    }
}

/// Checks if a label is enclosed in single quotes.
///
/// # Arguments
/// * `label` - The label string to check
///
/// # Returns
/// `true` if the label is enclosed by single quotes, `false` otherwise
///
/// # Examples
/// ```
/// # use nexwick::parser::utils::is_single_quoted;
/// assert_eq!(is_single_quoted("Pukeko"), false);
/// assert_eq!(is_single_quoted("'Swamp hen'"), true);
/// assert_eq!(is_single_quoted("'Baillon''s_crake'"), true);
/// assert_eq!(is_single_quoted("'Baillon's crake'"), true); // single quoted but not fully escaped
/// ```
pub fn is_single_quoted(label: &str) -> bool {
    label.starts_with('\'') && label.ends_with('\'') && label.len() >= 2
}

/// Escapes a label for safe use in NEXUS and Newick formats.
///
/// Labels containing special characters (punctuation, delimiters) are
/// wrapped in single quotes. Internal single quotes are escaped by doubling them.
/// Spaces are replaced with underscores in unescaped labels.
/// If the label is already escaped (in single quotes), it is returned as-is.
///
/// # Arguments
/// * `label` - The label string to escape
///
/// # Returns
/// An escaped label string safe for use in NEXUS and Newick files
///
/// # Examples
/// ```
/// # use nexwick::parser::utils::escape_label;
/// assert_eq!(escape_label("Pukeko"), "Pukeko");
/// assert_eq!(escape_label("Pu[ke]ko"), "'Pu[ke]ko'");
/// assert_eq!(escape_label("Australasian Swamphen"), "Australasian_Swamphen");
/// assert_eq!(escape_label("Australasian_Swamphen"), "Australasian_Swamphen");
/// assert_eq!(escape_label("'Australasian Swamphen'"), "'Australasian Swamphen'");
/// assert_eq!(escape_label("'Baillon''s_Crake'"), "'Baillon''s_Crake'");
/// assert_eq!(escape_label("'Baillon's Crake'"), "'Baillon''s Crake'");
/// ```
pub fn escape_label(label: &str) -> String {
    // Don't double-escape
    if is_escaped(label) {
        return label.to_string();
    }

    // Don't double single quote, but
    if is_single_quoted(label) {
        // ... fix any unescaped internal single quote
        let inner = &label[1..label.len() - 1];
        let mut fixed = String::with_capacity(inner.len() + 3);
        let mut chars = inner.chars().peekable();

        fixed.push('\'');
        while let Some(ch) = chars.next() {
            fixed.push(ch);
            if ch == '\'' {
                if chars.peek() == Some(&'\'') {
                    // If next char is also a single quote (already escaped), consume and push it
                    fixed.push(chars.next().unwrap());
                } else {
                    // But else, add the escaping quote
                    fixed.push('\'');
                }
            }
        }
        fixed.push('\'');

        return fixed;
    }

    if label.chars().any(|c| {
        matches!(
            c,
            ',' | ';' | '\t' | '\n' | '\r' | '(' | ')' | ':' | '[' | ']' | '\''
        )
    }) {
        // If contains special character, then replace single quotes with double single quotes
        let escaped = label.replace('\'', "''");
        // ... and wrap in single quotes
        format!("'{}'", escaped)
    } else {
        // Else just replace spaces with underscores
        label.replace(' ', "_")
    }
}

/// Unescapes a label that was escaped for NEXUS/Newick format.
///
/// Removes surrounding single quotes if present and then converts doubled single quotes
/// back to single quotes, and replaces underscores with spaces.
///
/// # Arguments
/// * `label` - The escaped label string
///
/// # Returns
/// The unescaped label string
///
/// # Examples
/// ```
/// # use nexwick::parser::utils::unescape_label;
/// assert_eq!(unescape_label("Pukeko"), "Pukeko");
/// assert_eq!(unescape_label("Pu[ke]ko"), "Pu[ke]ko");
/// assert_eq!(unescape_label("Australasian_Swamphen"), "Australasian Swamphen");
/// assert_eq!(unescape_label("'Australasian Swamphen'"), "Australasian Swamphen");
/// assert_eq!(unescape_label("'Australasian_Swamphen'"), "Australasian Swamphen");
/// assert_eq!(unescape_label("'Baillon''s_Crake'"), "Baillon's Crake");
/// ```
#[allow(dead_code)]
pub fn unescape_label(label: &str) -> String {
    let unquoted = if is_single_quoted(label) {
        // Remove quotes and unescape internal quotes
        label[1..label.len() - 1].replace("''", "'")
    } else {
        label.to_string()
    };
    // Replace underscores with spaces
    unquoted.replace("_", " ")
}

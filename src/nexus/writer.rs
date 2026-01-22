//! NEXUS format file writer (for tree model [`CompactTree`] +[`LeafLabelMap`]).

use crate::nexus::defs::{BLOCK_BEGIN, BLOCK_END, DIMENSIONS, NEXUS_HEADER, NTAX, TAXA, TAXLABELS, TRANSLATE, TREE, TREES};
use crate::newick::writer::{estimate_newick_len, to_newick_with_capacity, NewickStyle};
use crate::parser::utils::escape_label;
use crate::model::{CompactTree, LeafLabelMap};
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};

// =#========================================================================#=
// NEXUS WRITER
// =#========================================================================#=
/// Writer for phylogenetic trees ([`CompactTree`] +[`LeafLabelMap`])
/// in NEXUS format.
///
/// [`NexusWriter`] provides a buffered writer for creating NEXUS files
/// containing one or more phylogenetic trees with a shared leaf label mapping.
///
/// # Format Structure
/// The writer produces a NEXUS file with the following structure:
/// - `#NEXUS` header
/// - `TAXA` block with dimensions and tax labels
/// - `TREES` block with TRANSLATE command and tree definitions
///
/// # Example
/// ```ignore
/// use newick::nexus::NexusWriter;
/// use std::fs::File;
///
/// let file = File::create("output.trees")?;
/// let mut writer = NexusWriter::new(file);
/// writer.write_nexus(&trees, &label_map)?;
/// ```
pub struct NexusWriter {
    bw: BufWriter<File>,
}

// ============================================================================
// API (public)
// ============================================================================
impl NexusWriter {
    /// Creates a new NEXUS writer for the given file.
    ///
    /// # Arguments
    /// * `file` - The file to write to
    pub fn new(file: File) -> NexusWriter {
        NexusWriter {
            bw: BufWriter::new(file),
        }
    }

    /// Writes a complete NEXUS file with trees and their label mapping
    /// using integer keys (1-indexed) in TRANSLATE command.
    ///
    /// # Arguments
    /// * `trees` - Vector of trees to write
    /// * `leaf_label_map` - Shared leaf label mapping for all trees
    ///
    /// # Errors
    /// Returns an I/O error if writing fails
    pub fn write_nexus(&mut self, trees: &Vec<CompactTree>, leaf_label_map: &LeafLabelMap) -> io::Result<()> {
        self.header()?
            .taxa_block(leaf_label_map)?
            .trees_block(trees, leaf_label_map)?;
        Ok(())
    }
}

// ============================================================================
// Nexus Block & Command Writing (private)
// ============================================================================
impl NexusWriter {
    /// Writes the NEXUS file header ("#NEXUS"), returning itself for chaining.
    fn header(&mut self) -> io::Result<&mut Self> {
        self.write_all(NEXUS_HEADER)?.newline()?;
        Ok(self)
    }

    /// Writes the TAXA block with dimensions and taxon labels, returning itself for chaining.
    fn taxa_block(&mut self, map: &LeafLabelMap) -> io::Result<&mut Self> {
        // "Begin TAXA;"
        self.write_all(BLOCK_BEGIN)?
            .space()?
            .write_all(TAXA)?
            .semicolon_ln()?;

        // "\tDimensions ntaxa=n;"
        self.tab()?.write_all(DIMENSIONS)?
            .space()?
            .write_all(NTAX)?
            .equals()?
            .write_all(map.num_labels().to_string().as_bytes())?
            .semicolon_ln()?;

        // "\tTaxlabels [label ...];"
        self.tab()?
            .write_all(TAXLABELS)?;
        for label in map.labels() {
            let escaped_label = escape_label(label);
            self.space()?
                .write_all(escaped_label.as_bytes())?;
        }
        self.newline()?;

        // "End;"
        self.write_all(BLOCK_END)?
            .newline()?;

        Ok(self)
    }

    /// Writes the TREES block with TRANSLATE command and tree list, returning itself for chaining.
    fn trees_block(&mut self, trees: &Vec<CompactTree>, leaf_label_map: &LeafLabelMap) -> io::Result<&mut Self> {
        // - "Begin TREES;"
        self.write_all(BLOCK_BEGIN)?.space()?.write_all(TREES)?.semicolon_ln()?;

        self.translate_cmd(leaf_label_map)?
            .trees_cmd_list(trees)?;

        Ok(self)
    }

    /// Writes the TRANSLATE command mapping indices to labels, returning itself for chaining.
    fn translate_cmd(&mut self, leaf_label_map: &LeafLabelMap) -> io::Result<&mut Self> {
        // - "TRANSLATE [<key, label], ...];
        self.tab()?
            .write_all(TRANSLATE)?
            .newline()?;

        let num_labels = leaf_label_map.num_labels();
        let mut i = 0;

        for (label, id) in leaf_label_map.map() {
            i += 1;

            // "(id + 1) escaped_label,\n"
            let escaped_label = escape_label(label);

            self.tab()?.tab()?
                .write_all((id + 1).to_string().as_bytes())?
                .space()?
                .write_all(escaped_label.as_bytes())?;

            // No comma after last pair
            if i < num_labels {
                self.comma()?;
            }
            self.newline()?;
        }
        self.semicolon()?.newline()?;

        // - "End;"
        self.write_all(BLOCK_END)?
            .newline()?;

        Ok(self)
    }

    /// Writes the list of TREE commands in Newick format, returning itself for chaining.
    fn trees_cmd_list(&mut self, trees: &Vec<CompactTree>) -> io::Result<&mut Self> {
        if trees.is_empty() {
            return Ok(self);
        }

        // Estimate Newick string length
        let some_tree = &trees[0];
        let estimated_length = estimate_newick_len(&NewickStyle::OneIndexed, some_tree, None);

        // "TREE <name> = <Newick;>
        let mut i = 0;
        for tree in trees {
            let name = tree.name().map(|s| s.to_string())
                .unwrap_or_else(|| format!("tree_{}", i));

            self.write_all(TREE)?
                .space()?
                .write_all(name.as_bytes())?
                .space()?
                .equals()?
                .space()?
                .write_all(to_newick_with_capacity(&NewickStyle::OneIndexed, tree, None, estimated_length).as_bytes())?;

            i += 1;
        }

        Ok(self)
    }
}

// ============================================================================
// Little Helpers (private)
// ============================================================================
impl NexusWriter {
    /// Appends a byte slice to the [BufWriter], returning itself for chaining.
    fn write_all(&mut self, buf: &[u8]) -> io::Result<&mut Self> {
        self.bw.write_all(buf)?;
        Ok(self)
    }

    /// Appends a space character (' ') to the [BufWriter], returning itself for chaining.
    fn space(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b" ")?;
        Ok(self)
    }

    /// Appends a tab character ('\t') to the [BufWriter], returning itself for chaining.
    fn tab(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b"\t")?;
        Ok(self)
    }

    /// Appends a newline character ('\n') to the [BufWriter], returning itself for chaining.
    fn newline(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b"\n")?;
        Ok(self)
    }

    /// Appends a semicolon (';') to the [BufWriter], returning itself for chaining.
    fn semicolon(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b";")?;
        Ok(self)
    }

    /// Appends a semicolon followed by a newline (';\n') to the [BufWriter], returning itself for chaining.
    fn semicolon_ln(&mut self) -> io::Result<&mut Self> {
        self.semicolon()?.newline()?;
        Ok(self)
    }

    /// Appends a comma (',') to the [BufWriter], returning itself for chaining.
    fn comma(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b",")?;
        Ok(self)
    }

    /// Appends an equals sign ('=') to the [BufWriter], returning itself for chaining.
    fn equals(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b"=")?;
        Ok(self)
    }
}



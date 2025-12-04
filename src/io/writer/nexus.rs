use crate::model::leaf_label_map::LeafLabelMap;
use crate::model::tree::{NewickStyle, Tree};
use crate::parser::nexus::defs::{BLOCK_BEGIN, BLOCK_END, DIMENSIONS, NEXUS_HEADER, NTAX, TAXA, TAXLABELS, TRANSLATE, TREE};
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};

// =#========================================================================#=
// NEXUS WRITER
// =#========================================================================#=
pub struct NexusWriter {
    bw: BufWriter<File>,
}

// ============================================================================
// API
// ============================================================================
impl NexusWriter {
    pub fn new(file: File) -> NexusWriter {
        NexusWriter {
            bw: BufWriter::new(file),
        }
    }

    pub fn write_nexus(&mut self, trees: &Vec<Tree>, leaf_label_map: &LeafLabelMap) -> io::Result<()> {
        self.header()?
            .taxa_block(leaf_label_map)?
            .trees_block(trees, leaf_label_map)?;
        Ok(())
    }
}

// ============================================================================
// Nexus Block & Command Writing
// ============================================================================
impl NexusWriter {
    fn header(&mut self) -> io::Result<&mut Self> {
        // "Nexus\n"
        self.write_all(NEXUS_HEADER)?.newline()?;
        Ok(self)
    }

    fn taxa_block(&mut self, map: &LeafLabelMap) -> io::Result<&mut Self> {
        // - "Begin TAXA;"
        self.write_all(BLOCK_BEGIN)?
            .space()?
            .write_all(TAXA)?
            .semicolon_ln()?;

        // - "\tDimensions ntaxa=n;"
        self.tab()?.write_all(DIMENSIONS)?
            .space()?
            .write_all(NTAX)?
            .equals()?
            .write_all(map.num_labels().to_string().as_bytes())?
            .semicolon_ln()?;

        // - "\tTaxlabels [label ...];"
        self.tab()?
            .write_all(TAXLABELS)?;
        for label in map.labels() {
            let escaped_label = escape_label(label);
            self.space()?
                .write_all(escaped_label.as_bytes())?;
        }
        self.newline()?;

        // - "End;"
        self.write_all(BLOCK_END)?
            .newline()?;

        Ok(self)
    }

    fn trees_block(&mut self, trees: &Vec<Tree>, leaf_label_map: &LeafLabelMap) -> io::Result<&mut Self> {
        // - "Begin TREES;"
        self.write_all(BLOCK_BEGIN)?.space()?.write_all(TAXA)?.semicolon_ln()?;

        self.translate_cmd(leaf_label_map)?
            .trees_cmd_list(trees)?;

        Ok(self)
    }

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

    fn trees_cmd_list(&mut self, trees: &Vec<Tree>) -> io::Result<&mut Self> {
        // - "TREE <name> = <Newick;>
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
                .write_all(tree.to_newick(NewickStyle::OneIndexed, None).as_bytes())?;

            i += 1;
        }

        Ok(self)
    }
}

// ============================================================================
// Little Helpers
// ============================================================================
impl NexusWriter {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<&mut Self> {
        self.bw.write_all(buf)?;
        Ok(self)
    }

    fn space(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b" ")?;
        Ok(self)
    }

    fn tab(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b"\t")?;
        Ok(self)
    }

    fn newline(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b"\n").unwrap();
        Ok(self)
    }

    fn semicolon(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b";")?;
        Ok(self)
    }

    fn semicolon_ln(&mut self) -> io::Result<&mut Self> {
        self.semicolon()?.newline()?;
        Ok(self)
    }

    fn comma(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b",")?;
        Ok(self)
    }

    fn equals(&mut self) -> io::Result<&mut Self> {
        self.bw.write_all(b"=")?;
        Ok(self)
    }
}

fn escape_label(label: &String) -> String {
    if label.chars().any(|c| matches!(c, ' ' | ',' | ';' | '\t' | '\n' | '\r' | '(' | ')' | ':' | '[' | ']' | '\'')) {
        // Replace single quotes with double single quotes (SQL-style escaping)
        let escaped = label.replace('\'', "''");
        // Wrap in single quotes
        format!("'{}'", escaped)
    } else {
        label.to_string()
    }
}

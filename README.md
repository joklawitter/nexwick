# Nexwick
Rust library providing Nexus and Newick parsers to read in phylogenetic tree files and strings.

## Parser
### NEXUS Format
Parses trees from a NEXUS file based on the TAXA block and TREES block (including TRANSLATE command), ignoring other blocks.

### Newick Strings
Parses Newick strings with (optional) branch lengths. Does not handle extra data in vertices yet (e.g. `[@...]`).

### Design
Uses a mapping from leaves in the tree structure to taxa names instead of saving labels multiple times (since in posterior samples we might have thousands of trees).

# Nexwick
Rust library providing **Nex**us and Ne**wick** parsers to read in phylogenetic tree files and strings.


## Installation

`cargo add nexwick`  or add to your Cargo.toml:
```sh
[dependencies]
nexwick = "0.1"  
```

## Quick Start
```rust
use nexwick::{parse_newick_str, parse_nexus_file};
// Parse a Newick string
let tree = parse_newick_str("((A:0.1,B:0.2):0.3,C:0.4);").unwrap();
assert_eq!(tree.num_leaves(), 3);
// Parse a Nexus file
let (trees, labels) = parse_nexus_file("phylo.trees").unwrap();
```

## Documentation

See https://docs.rs/nexwick for full documentation, including:
- Tree types ([CompactTree] vs [SimpleTree])
- Parser configuration (burnin, lazy/eager mode)
- Custom tree builders

##   License

Licensed under either of Apache License 2.0 or MIT license at your option.
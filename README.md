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

## Performance

`nexwick` is built around a dependency-free, byte-stream parser and is designed
to be fast on large MCMC tree samples. In a benchmark across four real-world
BEAST2 datasets, it was the fastest of the tested Nexus parsers on every file,
in both lazy and eager modes. Relative to `nexwick` (baseline 1.0), the other
parsers ranged from roughly 1.5x to over 100x slower:

| Parser | Relative slowdown |
|:---|---:|
| `nexwick` | 1.0 |
| `cyanea` (Rust) | 1.5 – 4.5x |
| `ncl` (C++) | 3.3 – 5.4x |
| `ape` (R) | 8.6 – 14.6x |
| `BEAST2` (Java) | 11 – 23x |
| `DendroPy` (Python) | 44 – 150x |
| `Biopython` (Python) | 83 – 110x |

Full methodology, datasets, and per-file results are available in the
[nexwick_benchmark](https://github.com/joklawitter/nexwick_benchmark) repository.

##   License

Licensed under either of Apache License 2.0 or MIT license at your option.

## Contributing & Feedback

Feedback to `nexwick` is very welcome; I'm happy to expand capabilities based on your needs and wishes.

- **Found a bug?** Please just contact me directly or open an [issue](https://github.com/joklawitter/nexwick/issues) with a minimal example, ideally the offending tree string or file. 
- **Missing a feature?** Since Nexus and Newick are rich formats, not everything is supported yet, e.g. no internal vertex labels. So if there's a construct or option you need that isn't yet supported, don't shy away from opening a feature request or contacting me directly.

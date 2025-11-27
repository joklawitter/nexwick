use nexus_parser::parse_nexus_file;

fn main() {
    // let file = "path/to/your/trees/file/foo.trees";
    let file = "D:/Projects/Phylo/CCP/data/real/rsv2/reps/rep1/RSV2long.trees";
    // let start = Instant::now();
    print!("Parse file: {file}");
    let (trees, _) = parse_nexus_file(file).unwrap();
    // let duration = start.elapsed();
    // println!("Parsing took: {:?}", duration);
    print!("num trees: {}", trees.len());
}

use nexwick::parse_nexus_file;

fn main() {
    let path = "path/to/your/trees/file.trees";
    // let start = Instant::now();
    print!("Parse file: {path}\n");
    let (trees, _) = parse_nexus_file(path).unwrap();
    // let duration = start.elapsed();
    // println!("Parsing took: {:?}", duration);
    print!("num trees: {}", trees.len());
}

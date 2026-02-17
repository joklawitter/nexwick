use criterion::{Criterion, criterion_group, criterion_main};
use nexwick::nexus::NexusParserBuilder;

const REGRESSION_NEXUS_FILES: &[(&str, &str)] = &[
    ("RSV2", "benches/fixtures/RSV2-n129-1k.trees"),
    ("Yule50", "benches/fixtures/yule-n50-1k.trees"),
    ("DS1", "benches/fixtures/ds1-n27-1k.trees"),
];

const REPORTING_NEXUS_FILES: &[(&str, &str)] = &[
    ("RSV2", "benches/fixtures/RSV2-n129-1k.trees"),
    ("IECoR", "benches/fixtures/IECoR.trees"),
    ("Sunfish", "benches/fixtures/Sunfish.trees"),
];

fn parse_nexus_lazy(path: &str) {
    let mut parser = NexusParserBuilder::for_file(path)
        .unwrap()
        .lazy()
        .build()
        .unwrap();

    while let Some(_tree) = parser.next_tree().unwrap() {
        // consume all trees
    }
}

fn nexus_parsing_io(c: &mut Criterion) {
    for (name, path) in REGRESSION_NEXUS_FILES {
        c.bench_function(name, |b| {
            b.iter(|| parse_nexus_lazy(path));
        });
    }
}

fn nexus_reporting(c: &mut Criterion) {
    for (name, path) in REPORTING_NEXUS_FILES {
        c.bench_function(name, |b| {
            b.iter(|| parse_nexus_lazy(path));
        });
    }
}

criterion_group!(regression, nexus_parsing_io);
criterion_group! {
    name = reporting;
    config = Criterion::default().sample_size(10);
    targets = nexus_reporting
}
criterion_main!(regression, reporting);

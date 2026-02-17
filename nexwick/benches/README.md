# Benchmarks

Benchmarking the Nexus parser is split into two benchmark groups, one for regression benchmarking and one for reporting and comparison between parsing libraries; both using the [Criterion](https://crates.io/crates/criterion) crate. All benchmark files are posterior samples from BEAST2 runs. 

## Reporting benchmarks

Goal of this benchmark set is to make comparison between parsers/libraries possible as well as reporting on results of Nexwick.

**Settings:** 
- `sample_size(10)` to keep runtime manageable on large files.
- lazy parsing, buffered file reading, no skipping first or burnin

**Run:** `cargo bench -- reporting`

### Files

| Name          | Taxa | Trees | Size  | Data location                                                     | Reference                                                                            |
|---------------|------|-------|-------|-------------------------------------------------------------------|--------------------------------------------------------------------------------------|
| RSV2-n129-1k  | 129  | 10k   | 5.4MB | `/fixtures` (Repo)                                                | Run of this [BEAST2 Tutorial](https://taming-the-beast.org/tutorials/MEP-tutorial/)  |
| Sunfish       | 61   | 7.5k  | 40MB  | [Link](https://datadryad.org/dataset/doi:10.5061/dryad.kprr4xh45) | [Paper](https://doi.org/10.1016/j.ympev.2021.107156)                                 |
| IE-Languages  | 161  | 37k   | 568MB | [Link](https://share.eva.mpg.de/index.php/s/E4Am2bbBA3qLngC)      | [Paper](https://www.science.org/doi/abs/10.1126/science.abg0818)                     |


## Regression benchmarks

Goal is to track performance regression during development, with small but varying files.

**Settings:** 
- `sample_size(100)`, the default.
- lazy parsing, buffered file reading, no skipping first or burnin

**Run:** `cargo bench -- regression`

### Files

| Name         | Taxa | Trees | Size  | Data location      | Reference                                                                                                                                                   |
|--------------|------|-------|-------|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------|
| DS1-n27-1k | 27 | 1k | 1.2MB | `/fixtures` (Repo) | Run for [paper](https://doi.org/10.1371/journal.pcbi.1012789) based on [this original data](https://doi.org/10.1093/oxfordjournals.molbev.a040628) |  
| Yule-n50-1k  | 50   | 1k    | 2.2MB | `/fixtures` (Repo) | [Simulated data](https://doi.org/10.17608/k6.auckland.27041803) for [paper](https://www.biorxiv.org/content/10.1101/2024.09.25.615070v1)           |
| RSV2-n129-1k | 129  | 1k    | 5.3MB | `/fixtures` (Repo) | Run of this [BEAST2 Tutorial](https://taming-the-beast.org/tutorials/MEP-tutorial/)                                                                         |



## Adding a new benchmark file
To add a new benchmark file, simple add its name and path to in the appropriate list (constant) in `benchmark.rs` (`REGRESSION_NEXUS_FILES` or `REPORTING_NEXUS_FILES`).
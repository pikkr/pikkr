# Pikkr

[![Crates.io version shield](https://img.shields.io/crates/v/pikkr.svg)](https://crates.io/crates/pikkr)

JSON parser which picks up values directly without performing tokenization in Rust

## Abstract

Pikkr is a JSON parser which picks up values directly without performing tokenization in Rust. This JSON parser is implemented based on [Y. Li, N. R. Katsipoulakis, B. Chandramouli, J. Goldstein, and D. Kossmann. Mison: a fast JSON parser for data analytics. In *VLDB*, 2017](http://www.vldb.org/pvldb/vol10/p1118-li.pdf).

This JSON parser extracts values from a JSON record without using finite state machines (FSMs) and performing tokenization. It parses JSON records in the following procedures:

1. [Indexing] Creates an index which maps logical locations of queried fields to their physical locations by using SIMD instructions and bit manipulation.
2. [Basic parsing] Finds values of queried fields by scanning a JSON record using the index created in the previous process and learns their logical locations (i.e. pattern of the JSON structure) in the early stages.
3. [Speculative parsing] Speculates logical locations of queried fields by using the learned result information, jumps directly to their physical locations and extracts values in the later stages. Fallbacks to basic parsing if the speculation fails.

This JSON parser performs well when there are a limited number of different JSON structural variants in a JSON data stream or JSON collection, and that is a common case in data analytics field.

Please read the paper mentioned in the opening paragraph for the details of the JSON parsing algorithm.

## Performance

### Benchmark Result

![](https://raw.githubusercontent.com/pikkr/pikkr/master/img/benchmark.png)

### Hardware

```
Model Name: MacBook Pro
Processor Name: Intel Core i7
Processor Speed: 3.3 GHz
Number of Processors: 1
Total Number of Cores: 2
L2 Cache (per Core): 256 KB
L3 Cache: 4 MB
Memory: 16 GB
```

### Crates

* [serde_json](https://crates.io/crates/serde_json) 1.0.3
* [json](https://crates.io/crates/json) 0.11.9
* [pikkr](https://crates.io/crates/pikkr) 0.8.0

### JSON Data

* "a JSON data set of startup company information" on [JSON Data Sets | JSON Studio](http://jsonstudio.com/resources/).

### Benchmark Code

* [pikkr/rust-json-parser-benchmark: Rust JSON Parser Benchmark](https://github.com/pikkr/rust-json-parser-benchmark)

## Example

### Code

```rust
extern crate pikkr;

fn main() {
    let queries = vec![
        "$.f1".as_bytes(),
        "$.f2.f1".as_bytes(),
    ];
    let train_num = 2; // Number of records used as training data
                       // before Pikkr starts speculative parsing.
    let mut p = match pikkr::Pikkr::new(&queries, train_num) {
        Ok(p) => p,
        Err(err) => panic!("There was a problem creating a JSON parser: {:?}", err.kind()),
    };
    let recs = vec![
        r#"{"f1": "a", "f2": {"f1": 1, "f2": true}}"#,
        r#"{"f1": "b", "f2": {"f1": 2, "f2": true}}"#,
        r#"{"f1": "c", "f2": {"f1": 3, "f2": true}}"#, // Speculative parsing starts from this record.
        r#"{"f2": {"f2": true, "f1": 4}, "f1": "d"}"#,
        r#"{"f2": {"f2": true, "f1": 5}}"#,
        r#"{"f1": "e"}"#
    ];
    for rec in recs {
        match p.parse(rec.as_bytes()) {
            Ok(results) => {
                for result in results {
                    print!("{} ", match result {
                        Some(result) => String::from_utf8(result.to_vec()).unwrap(),
                        None => String::from("None"),
                    });
                }
                println!();
            },
            Err(err) => println!("There was a problem parsing a record: {:?}", err.kind()),
        }
    }
    /*
    Output:
        "a" 1
        "b" 2
        "c" 3
        "d" 4
        None 5
        "e" None
    */
}
```

### Build

```bash
$ cargo --version
cargo 0.22.0-nightly (3d3f2c05d 2017-08-27) # Make sure that nightly release is being used.
$ RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Run

```bash
$ ./target/release/[package name]
"a" 1
"b" 2
"c" 3
"d" 4
None 5
"e" None
```

## Documentation

* [pikkr - Rust](https://pikkr.github.io/doc/pikkr/)

## Restrictions

* [Rust nightly channel](https://github.com/rust-lang-nursery/rustup.rs/blob/master/README.md#working-with-nightly-rust) and [CPUs with AVX2](https://en.wikipedia.org/wiki/Advanced_Vector_Extensions#CPUs_with_AVX2) are needed to build Rust source code which depends on Pikkr and run the executable binary file because Pikkr uses AVX2 Instructions.

## Contributing

Any kind of contribution (e.g. comment, suggestion, question, bug report and pull request) is welcome.

#!/bin/bash -eu

export RUSTFLAGS="-C target-cpu=native"

cargo install rustfmt-nightly
cargo fmt -- --write-mode=diff
cargo build --release
cargo test --release

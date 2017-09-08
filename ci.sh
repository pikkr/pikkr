#!/bin/bash -eu

export RUSTFLAGS="-C target-cpu=native"

rustfmt_installed=$(cargo install --list | grep rustfmt-nightly | wc -l)

if [ $rustfmt_installed -eq 0 ]; then
    cargo install rustfmt-nightly
fi
cargo fmt -- --write-mode=diff
cargo build --release
cargo test --release

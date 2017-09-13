#!/bin/bash -eu

MODE="${1:-}"
FEATURES="${2:-}"
TOOLCHAIN="${3:-nightly}"

function ensure_installed () {
    local installed=$(cargo install --list | grep "$1" | wc -l)
    if [ $installed -eq 0 ]; then
        cargo install "$1"
    fi
}

case "$MODE" in
    "")
        # The default script for testing.
        cargo build --release --features "$FEATURES"
        cargo test --release --features "$FEATURES"
        if [ "$TOOLCHAIN" == "nightly" ]; then
            cargo bench --features "$FEATURES"
        fi
        ;;

    format-diff)
        ensure_installed "rustfmt-nightly"
        cargo fmt -- --write-mode=diff
        ;;

    clippy)
        ensure_installed "clippy"
        cargo clippy --features "$FEATURES"
        ;;

    *)
        echo "Unknown checker: '$MODE'"
        exit 1
        ;;
esac

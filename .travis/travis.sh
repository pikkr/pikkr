#!/bin/bash -eu

MODE="${1:-}"
FEATURES="${2:-default}"
TOOLCHAIN="${3:-nightly}"

function ensure_installed () {
    local installed=$(cargo install --list | grep "$1" | wc -l)
    if [ $installed -eq 0 ]; then
        cargo install "$1"
    fi
}

function run_cargo () {
    local args=("$@")
    echo "=== cargo ${args[@]} ==="
    cargo "${args[@]}"
}

case "$MODE" in
    "")
        # The default script for testing.
        run_cargo build --release --features "$FEATURES"
        run_cargo test --release --features "$FEATURES"
        if [ "$TOOLCHAIN" == "nightly" ]; then
            run_cargo bench --features "$FEATURES"
        fi
        ;;

    format-diff)
        ensure_installed "rustfmt-nightly"
        run_cargo fmt -- --write-mode=diff
        ;;

    clippy)
        ensure_installed "clippy"
        run_cargo clippy --features "$FEATURES"
        ;;

    *)
        echo "Unknown checker: '$MODE'"
        exit 1
        ;;
esac

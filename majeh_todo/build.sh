#!/bin/bash
# Build the Leptos Todo Application for WebAssembly
# Usage: ./build.sh
# Note: Builds for WASM target which avoids system GUI library requirements

set -e

echo "=== Building Maheh Todo Application (WebAssembly) ==="

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Are you in the project root?"
    exit 1
fi

# Build for WebAssembly target (no system GUI libs needed)
cargo build --target wasm32-unknown-unknown

if [ $? -eq 0 ]; then
    echo ""
    echo "Build completed successfully!"
    echo "Wasm output: target/wasm32-unknown-unknown/debug/majeh_todo.wasm"
    echo ""
    echo "To serve the app:"
    echo "./run.sh"
else
    echo ""
    echo "Build failed. For WebAssembly (recommended):"
    echo "  rustup target add wasm32-unknown-unknown"
    exit 1
fi
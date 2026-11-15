#!/bin/bash
# Prerequisites checker for Maheh Todo Application
set -e

echo "=== Maheh Todo Prerequisites Checker ==="
echo ""

# Check Rust
echo "Checking Rust compiler..."
if command -v rustc &>/dev/null; then
    echo "✅ Rust found: $(rustc --version)"
else
    echo "❌ Rust not found"
    echo "Installing Rust via rustup..."
    curl --proto 'https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s - -y
    source "$HOME/.cargo/env"
    echo "✅ Rust installed: $(rustc --version)"
fi

# Check Cargo
echo ""
echo "Checking Cargo..."
if command -v cargo &>/dev/null; then
    echo "✅ Cargo found: $(cargo --version)"
else
    echo "❌ Cargo not found"
    exit 1
fi

# Check WebAssembly target
echo ""
echo "Checking WebAssembly target..."
if rustup target list 2>/dev/null | grep -q "wasm32-unknown-unknown"; then
    echo "✅ wasm32-unknown-unknown target available"
else
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
    echo "✅ wasm32-unknown-unknown target installed"
fi

echo ""
echo "=== All Prerequisites Checked ==="
echo "You can now run:"
echo "  ./build.sh   - Build the project"
echo "  ./run.sh     - Start server at http://localhost:8080"
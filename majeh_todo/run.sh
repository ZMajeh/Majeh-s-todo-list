#!/bin/bash
# Run the Leptos Todo Application Server using Trunk
# Usage: ./run.sh

set -e

echo "=== Starting Maheh Todo Application Server (Trunk) ==="
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found. Are you in the project root?"
    exit 1
fi

echo "Starting Trunk development server on port 8080..."
echo ""
echo "App available at: http://localhost:8080"
echo "Login page: http://localhost:8080/login"
echo "Dashboard: http://localhost:8080/"
echo ""
echo "Press Ctrl+C to stop the server"

trunk serve --port 8080 --address 0.0.0.0

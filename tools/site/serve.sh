#!/bin/sh
# Build the York website with the York toolchain, then serve it locally.
# Usage:  ./serve.sh [port]
set -e
cd "$(dirname "$0")/site"

PORT="${1:-8000}"

york run main.yk

echo ""
echo "Serving site at http://localhost:$PORT"
echo "Press Ctrl+C to stop."
python3 -m http.server "$PORT"
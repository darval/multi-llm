#!/bin/bash
# Run complete test suite (format, clippy, unit, integration)
#
# Usage: ./scripts/test-all.sh [--skip-integration]

set -e

SKIP_INTEGRATION=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-integration)
            SKIP_INTEGRATION=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: ./scripts/test-all.sh [--skip-integration]"
            exit 1
            ;;
    esac
done

echo "🚀 Running complete test suite..."
echo ""

# Step 1: Format
echo "📝 Checking Rust formatting..."
cargo fmt --check
echo "✅ Format check passed"
echo ""

# Step 2: Clippy
echo "📎 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings
echo "✅ Clippy passed"
echo ""

# Step 3: Unit tests
echo "🧪 Running unit tests..."
./scripts/test-unit.sh
echo ""

# Step 4: Integration tests (unless skipped)
if [ "$SKIP_INTEGRATION" = false ]; then
    echo "🔗 Running integration tests..."
    ./scripts/test-integration.sh
    echo ""
else
    echo "⏭️  Skipping integration tests (--skip-integration flag set)"
    echo ""
fi

echo "🎉 Complete test suite passed!"
echo ""
echo "📊 Test Summary:"
echo "   ✅ Format check"
echo "   ✅ Clippy"
echo "   ✅ Unit tests (Rust + React)"
if [ "$SKIP_INTEGRATION" = false ]; then
    echo "   ✅ Integration tests (Rust + React)"
else
    echo "   ⏭️  Integration tests (skipped)"
fi

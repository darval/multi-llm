#!/bin/bash
# Pre-push check script - runs before pushing code
# Catches basic issues quickly (< 5 min target)
#
# Usage: ./scripts/pre-push-check.sh
# Or setup as git pre-push hook

set -e

echo "🔍 Running pre-push checks..."
echo ""

# Step 1: Format check
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
cargo test --lib --bins
echo "✅ Unit tests passed"
echo ""

echo "✨ All pre-push checks passed!"
echo ""
echo "💡 Tip: To run all tests including integration tests, use:"
echo "   ./scripts/test-all.sh"

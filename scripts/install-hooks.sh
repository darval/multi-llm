#!/bin/bash
# Install git hooks for the multi-llm project
#
# Usage: ./scripts/install-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "🔧 Installing git hooks..."
echo ""

# Pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
    echo "⚠️  Pre-commit hook already exists"
    read -p "   Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "   Skipping pre-commit hook"
    else
        cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook - runs quality checks and unit tests before allowing commit
#
# This hook runs automatically before each commit to catch issues early.
# Runs: format check, clippy, and unit tests
#
# To skip this hook temporarily (emergency commits):
#   git commit --no-verify
#
# To disable permanently:
#   rm .git/hooks/pre-commit

set -e
set -o pipefail

echo ""
echo "🔍 Running pre-commit checks..."
echo ""

# Step 1: Format check
echo "📝 Checking Rust formatting..."
if ! cargo fmt --check --quiet 2>&1; then
    echo "❌ Format check failed!"
    echo ""
    echo "Run: cargo fmt"
    echo "Then retry your commit."
    exit 1
fi
echo "✅ Format check passed"
echo ""

# Step 2: Clippy (production code only - tests have looser lint rules)
echo "📎 Running Clippy..."
if ! cargo clippy --lib --bins --all-features --quiet -- -D warnings 2>&1; then
    echo "❌ Clippy failed!"
    echo ""
    echo "Fix the warnings, then retry your commit."
    echo ""
    echo "💡 To see details: cargo clippy --lib --bins --all-features"
    echo "💡 To auto-fix: cargo clippy --fix --lib --bins --all-features"
    echo "💡 To skip this hook (emergency only): git commit --no-verify"
    exit 1
fi
echo "✅ Clippy passed"
echo ""

# Step 3: Unit tests (fast - ~5-6 seconds)
echo "🧪 Running unit tests..."
if ! ./scripts/test-unit.sh 2>&1 | grep -v "^🧪\|^🦀\|^⚛️\|^✨"; then
    echo "❌ Unit tests failed!"
    echo ""
    echo "Fix the failing tests, then retry your commit."
    echo ""
    echo "💡 To see full output: ./scripts/test-unit.sh --verbose"
    echo "💡 To skip this hook (emergency only): git commit --no-verify"
    exit 1
fi

echo ""
echo "✅ All pre-commit checks passed!"
echo ""
echo "💡 For integration tests before pushing:"
echo "   ./scripts/pre-push-check.sh"
echo ""
EOF
        chmod +x "$HOOKS_DIR/pre-commit"
        echo "   ✅ Pre-commit hook installed"
    fi
else
    cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook - runs quality checks and unit tests before allowing commit
#
# This hook runs automatically before each commit to catch issues early.
# Runs: format check, clippy, and unit tests
#
# To skip this hook temporarily (emergency commits):
#   git commit --no-verify
#
# To disable permanently:
#   rm .git/hooks/pre-commit

set -e
set -o pipefail

echo ""
echo "🔍 Running pre-commit checks..."
echo ""

# Step 1: Format check
echo "📝 Checking Rust formatting..."
if ! cargo fmt --check --quiet 2>&1; then
    echo "❌ Format check failed!"
    echo ""
    echo "Run: cargo fmt"
    echo "Then retry your commit."
    exit 1
fi
echo "✅ Format check passed"
echo ""

# Step 2: Clippy (production code only - tests have looser lint rules)
echo "📎 Running Clippy..."
if ! cargo clippy --lib --bins --all-features --quiet -- -D warnings 2>&1; then
    echo "❌ Clippy failed!"
    echo ""
    echo "Fix the warnings, then retry your commit."
    echo ""
    echo "💡 To see details: cargo clippy --lib --bins --all-features"
    echo "💡 To auto-fix: cargo clippy --fix --lib --bins --all-features"
    echo "💡 To skip this hook (emergency only): git commit --no-verify"
    exit 1
fi
echo "✅ Clippy passed"
echo ""

# Step 3: Unit tests (fast - ~5-6 seconds)
echo "🧪 Running unit tests..."
if ! ./scripts/test-unit.sh 2>&1 | grep -v "^🧪\|^🦀\|^⚛️\|^✨"; then
    echo "❌ Unit tests failed!"
    echo ""
    echo "Fix the failing tests, then retry your commit."
    echo ""
    echo "💡 To see full output: ./scripts/test-unit.sh --verbose"
    echo "💡 To skip this hook (emergency only): git commit --no-verify"
    exit 1
fi

echo ""
echo "✅ All pre-commit checks passed!"
echo ""
echo "💡 For integration tests before pushing:"
echo "   ./scripts/pre-push-check.sh"
echo ""
EOF
    chmod +x "$HOOKS_DIR/pre-commit"
    echo "✅ Pre-commit hook installed"
fi
echo ""

echo "✨ Git hooks installation complete!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Runs format check + clippy + unit tests"
echo ""
echo "To bypass hooks temporarily (emergency only):"
echo "  git commit --no-verify"
echo ""

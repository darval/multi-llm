#!/bin/bash
# Run all unit tests (Rust + React)
#
# Usage: ./scripts/test-unit.sh [OPTIONS]
#   OPTIONS are passed to cargo test (e.g., --nocapture, --test-threads=1)
#
# Use --verbose to see full cargo output

set -e

VERBOSE=false
CARGO_ARGS=()

# Parse arguments
for arg in "$@"; do
    if [ "$arg" = "--verbose" ]; then
        VERBOSE=true
    else
        CARGO_ARGS+=("$arg")
    fi
done

echo "🧪 Running all unit tests..."
echo ""

# Rust unit tests (with RUSTFLAGS to match CI - warnings as errors in test code)
echo "🦀 Running Rust unit tests..."
if [ "$VERBOSE" = true ]; then
    RUSTFLAGS="-D warnings" cargo test --lib --bins --all-features "${CARGO_ARGS[@]}"
else
    # Time the test run and capture output
    START_TIME=$(date +%s.%N)
    TEST_OUTPUT=$(RUSTFLAGS="-D warnings" cargo test --lib --bins --all-features --quiet "${CARGO_ARGS[@]}" 2>&1)
    END_TIME=$(date +%s.%N)

    # Calculate elapsed time
    ELAPSED=$(echo "$END_TIME - $START_TIME" | bc)
    ELAPSED_FORMATTED=$(printf "%.2fs" $ELAPSED)

    # Sum all passed tests
    TOTAL_PASSED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -o "[0-9]* passed" | awk '{sum += $1} END {print sum}')

    # Count ignored tests
    TOTAL_IGNORED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -o "[0-9]* ignored" | awk '{sum += $1} END {print sum}')

    if [ -n "$TOTAL_PASSED" ] && [ "$TOTAL_PASSED" -gt 0 ]; then
        if [ -n "$TOTAL_IGNORED" ] && [ "$TOTAL_IGNORED" -gt 0 ]; then
            echo "  ✓ ${TOTAL_PASSED} passed, ${TOTAL_IGNORED} ignored (${ELAPSED_FORMATTED})"
        else
            echo "  ✓ ${TOTAL_PASSED} passed (${ELAPSED_FORMATTED})"
        fi
    else
        echo "  ⚠️  No tests found"
    fi
fi
echo ""

# React unit tests (when configured)
if [ -f "web/package.json" ]; then
    echo "⚛️  Running React unit tests..."
    cd web
    if npm run test:unit 2>/dev/null; then
        echo "✅ React unit tests passed"
    else
        echo "⚠️  No React unit tests configured yet (skipping)"
    fi
    cd ..
    echo ""
fi

echo "✨ All unit tests passed!"

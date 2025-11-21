#!/bin/bash
# Run all integration tests
# Some tests may require external services (marked with #[ignore])
#
# Usage: ./scripts/test-integration.sh [OPTIONS]
#   OPTIONS are passed to cargo test (e.g., --nocapture)
#
# Use --verbose to see full cargo output
# Use --include-ignored to run tests that require external services

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

echo "🔗 Running integration tests..."
echo ""

# Check if tests directory exists
if [ ! -d "tests" ]; then
    echo "⚠️  No integration tests found yet (tests/ directory doesn't exist)"
    exit 0
fi

# Check if there are any test files
if ! ls tests/*.rs 1> /dev/null 2>&1; then
    echo "⚠️  No integration tests found yet (no .rs files in tests/)"
    exit 0
fi

# Run integration tests
echo "🦀 Running Rust integration tests..."
START_TIME=$(date +%s.%N)

if [ "$VERBOSE" = true ]; then
    # Verbose mode: show all output
    cargo test --tests "${CARGO_ARGS[@]}"
else
    # Non-verbose: capture for summary, but show all on failure
    if cargo test --tests "${CARGO_ARGS[@]}" 2>&1 | tee /tmp/integration-test-output.log; then
        # Tests passed - show concise summary
        END_TIME=$(date +%s.%N)
        ELAPSED=$(awk "BEGIN {print $END_TIME - $START_TIME}")
        ELAPSED_FORMATTED=$(printf "%.2fs" $ELAPSED)

        TOTAL_PASSED=$(grep "test result:" /tmp/integration-test-output.log | grep -o "[0-9]* passed" | awk '{sum += $1} END {print sum}')
        TOTAL_IGNORED=$(grep "test result:" /tmp/integration-test-output.log | grep -o "[0-9]* ignored" | awk '{sum += $1} END {print sum}')

        if [ -n "$TOTAL_PASSED" ] && [ "$TOTAL_PASSED" -gt 0 ]; then
            if [ -n "$TOTAL_IGNORED" ] && [ "$TOTAL_IGNORED" -gt 0 ]; then
                echo "  ✓ ${TOTAL_PASSED} tests passed, ${TOTAL_IGNORED} ignored (${ELAPSED_FORMATTED})"
            else
                echo "  ✓ ${TOTAL_PASSED} tests passed (${ELAPSED_FORMATTED})"
            fi
        else
            echo "  ⚠️  No integration tests found"
        fi
        rm -f /tmp/integration-test-output.log
    else
        # Tests failed - output already shown by tee, just exit
        echo "  ❌ Tests failed (see output above)"
        rm -f /tmp/integration-test-output.log
        exit 1
    fi
fi

echo ""
echo "✨ Integration tests passed!"
echo ""

if grep -q "ignored" /tmp/integration-test-output.log 2>/dev/null; then
    echo "💡 Some tests were ignored (likely require external services)"
    echo "   To run them: ./scripts/test-integration.sh --include-ignored"
fi

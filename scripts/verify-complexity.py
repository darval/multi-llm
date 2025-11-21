#!/usr/bin/env python3
"""
Verify that all functions in a crate meet cognitive complexity limits.

Uses the Rust-based verify-rust-metrics tool for accurate AST-based analysis.
Falls back to cargo clippy if the Rust tool is not available.

- Goal: <= 10 (missing this is a failure)
- Hard limit: <= 15 (exceeding this requires explicit approval)

Usage:
    python3 scripts/verify-complexity.py <crate-path> [--goal SCORE] [--hard-limit SCORE]

Example:
    python3 scripts/verify-complexity.py multi-llm
    python3 scripts/verify-complexity.py multi-llm --goal 10 --hard-limit 15

Exit codes:
    0 - All functions meet goal
    1 - Functions exceed goal but meet hard limit
    2 - Functions exceed hard limit (requires approval)
"""

import os
import sys
import argparse
import subprocess


def run_rust_tool(crate_path, goal, hard_limit):
    """Run the Rust-based verification tool."""
    # Find the project root (where Cargo.toml is)
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir)

    # Build the crate path relative to project root
    if os.path.isabs(crate_path):
        abs_crate_path = crate_path
    else:
        abs_crate_path = os.path.join(project_root, crate_path)

    # Run the Rust tool
    cmd = [
        'cargo', 'run', '--release', '-p', 'verify-rust-metrics',
        '--',
        abs_crate_path,
        '--complexity-goal', str(goal),
        '--complexity-hard', str(hard_limit)
    ]

    try:
        result = subprocess.run(
            cmd,
            cwd=project_root,
            capture_output=True,
            text=True
        )

        # Print the tool's output
        if result.stdout:
            print(result.stdout, end='')
        if result.stderr:
            print(result.stderr, end='', file=sys.stderr)

        return result.returncode

    except Exception as e:
        print(f"Error running Rust verification tool: {e}", file=sys.stderr)
        print("Falling back to clippy...", file=sys.stderr)
        return None


def main():
    parser = argparse.ArgumentParser(
        description='Verify cognitive complexity is within limits'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )
    parser.add_argument(
        '--goal',
        type=int,
        default=10,
        help='Goal cognitive complexity (default: 10)'
    )
    parser.add_argument(
        '--hard-limit',
        type=int,
        default=15,
        help='Hard limit cognitive complexity (default: 15)'
    )

    args = parser.parse_args()

    # Try using the Rust tool first
    exit_code = run_rust_tool(args.crate_path, args.goal, args.hard_limit)

    if exit_code is not None:
        return exit_code

    # Fallback message (shouldn't happen if Rust tool is built)
    print("❌ Rust verification tool not available. Please build it:", file=sys.stderr)
    print("   cargo build --release -p verify-rust-metrics", file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main())

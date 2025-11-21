#!/usr/bin/env python3
"""
Verify that all Rust files in a crate meet file size limits.

Checks both GOAL (500 lines) and HARD LIMIT (1000 lines) by default.

Usage:
    python3 scripts/verify-file-size.py <crate-path> [--goal LINES] [--hard-limit LINES]

Example:
    python3 scripts/verify-file-size.py multi-llm
    python3 scripts/verify-file-size.py multi-llm --goal 500 --hard-limit 1000

Exit codes:
    0 - All files meet goal
    1 - Files exceed goal but meet hard limit
    2 - Files exceed hard limit (requires approval)
"""

import os
import sys
import argparse


def check_file_sizes(crate_path, goal, hard_limit):
    """Check all .rs files in the crate for size violations."""
    file_sizes = []
    src_path = os.path.join(crate_path, 'src')

    if not os.path.exists(src_path):
        print(f"Error: {src_path} does not exist", file=sys.stderr)
        return None

    for root, dirs, files in os.walk(src_path):
        for file in files:
            if not file.endswith('.rs'):
                continue

            path = os.path.join(root, file)
            with open(path, 'r') as f:
                line_count = sum(1 for _ in f)

            if line_count >= goal:
                rel_path = os.path.relpath(path, crate_path)
                file_sizes.append((rel_path, line_count))

    # Separate by severity
    exceeds_hard_limit = [(p, s) for p, s in file_sizes if s >= hard_limit]
    exceeds_goal = [(p, s) for p, s in file_sizes if s >= goal and s < hard_limit]

    return exceeds_hard_limit, exceeds_goal


def main():
    parser = argparse.ArgumentParser(
        description='Verify Rust file sizes meet limits'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )
    parser.add_argument(
        '--goal',
        type=int,
        default=500,
        help='Goal file size in lines (default: 500)'
    )
    parser.add_argument(
        '--hard-limit',
        type=int,
        default=1000,
        help='Hard limit file size in lines (default: 1000)'
    )

    args = parser.parse_args()

    result = check_file_sizes(args.crate_path, args.goal, args.hard_limit)

    if result is None:
        return 2

    exceeds_hard_limit, exceeds_goal = result

    if not exceeds_hard_limit and not exceeds_goal:
        print(f"✓ All files < {args.goal} lines (goal)")
        return 0

    exit_code = 0

    if exceeds_hard_limit:
        print(f"❌ HARD LIMIT VIOLATION: {len(exceeds_hard_limit)} file(s) >= {args.hard_limit} lines:")
        print("   These require explicit approval to ignore.\n")
        for path, size in sorted(exceeds_hard_limit, key=lambda x: x[1], reverse=True):
            print(f"  {path}: {size} lines")
        exit_code = 2
        print()

    if exceeds_goal:
        print(f"{'⚠️ ' if not exceeds_hard_limit else ''}GOAL MISSED: {len(exceeds_goal)} file(s) >= {args.goal} lines but < {args.hard_limit} lines:")
        print("   These should be refactored but don't block progress.\n")
        for path, size in sorted(exceeds_goal, key=lambda x: x[1], reverse=True):
            print(f"  {path}: {size} lines (goal: {args.goal}, hard limit: {args.hard_limit})")
        if exit_code == 0:
            exit_code = 1

    return exit_code


if __name__ == '__main__':
    sys.exit(main())

#!/usr/bin/env python3
"""
Verify that all functions in Rust files meet function size limits.

Checks both GOAL (50 lines) and HARD LIMIT (100 lines) by default.

Usage:
    python3 scripts/verify-function-size.py <crate-path> [--goal LINES] [--hard-limit LINES]

Example:
    python3 scripts/verify-function-size.py multi-llm
    python3 scripts/verify-function-size.py multi-llm --goal 50 --hard-limit 100

Exit codes:
    0 - All functions meet goal
    1 - Functions exceed goal but meet hard limit
    2 - Functions exceed hard limit (requires approval)
"""

import os
import sys
import re
import argparse


def count_function_lines(file_path):
    """Find all functions in a file and return their sizes."""
    with open(file_path, 'r') as f:
        lines = f.readlines()

    functions = []
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        # Match function definitions (pub fn, async fn, etc.)
        # More comprehensive pattern to catch various function types
        match = re.match(r'(pub\s+)?(\(crate\)\s+)?(unsafe\s+)?(async\s+)?(const\s+)?fn\s+(\w+)', line)
        if match:
            func_start = i
            func_name = match.group(6)

            # Count braces while respecting strings and comments
            brace_count = count_braces_in_line(lines[i])
            i += 1

            # Count braces to find function end
            while i < len(lines) and brace_count > 0:
                brace_count += count_braces_in_line(lines[i])
                i += 1

            func_length = i - func_start
            functions.append((func_start + 1, func_name, func_length))
        else:
            i += 1

    return functions


def count_braces_in_line(line):
    """Count net braces in a line, ignoring those in strings and comments."""
    # Remove line comments first
    if '//' in line:
        line = line.split('//')[0]

    brace_count = 0
    state = {'in_string': False, 'in_char': False, 'escaped': False}

    for ch in line:
        brace_count += process_character(ch, state)

    return brace_count


def process_character(ch, state):
    """Process a single character and return brace count delta."""
    if state['escaped']:
        state['escaped'] = False
        return 0

    if ch == '\\' and (state['in_string'] or state['in_char']):
        state['escaped'] = True
        return 0

    if ch == '"' and not state['in_char']:
        state['in_string'] = not state['in_string']
        return 0

    if ch == "'" and not state['in_string']:
        state['in_char'] = not state['in_char']
        return 0

    if not state['in_string'] and not state['in_char']:
        if ch == '{':
            return 1
        if ch == '}':
            return -1

    return 0


def check_function_sizes(crate_path, goal, hard_limit):
    """Check all .rs files in the crate for function size violations."""
    all_violations = []
    src_path = os.path.join(crate_path, 'src')

    if not os.path.exists(src_path):
        print(f"Error: {src_path} does not exist", file=sys.stderr)
        return None

    for root, dirs, files in os.walk(src_path):
        for file in files:
            if not file.endswith('.rs'):
                continue

            path = os.path.join(root, file)
            rel_path = os.path.relpath(path, crate_path)

            functions = count_function_lines(path)
            for line_num, func_name, length in functions:
                if length >= goal:
                    all_violations.append((rel_path, line_num, func_name, length))

    # Separate by severity
    exceeds_hard_limit = [(p, l, n, s) for p, l, n, s in all_violations if s >= hard_limit]
    exceeds_goal = [(p, l, n, s) for p, l, n, s in all_violations if s >= goal and s < hard_limit]

    return exceeds_hard_limit, exceeds_goal


def main():
    parser = argparse.ArgumentParser(
        description='Verify Rust function sizes meet limits'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )
    parser.add_argument(
        '--goal',
        type=int,
        default=50,
        help='Goal function size in lines (default: 50)'
    )
    parser.add_argument(
        '--hard-limit',
        type=int,
        default=100,
        help='Hard limit function size in lines (default: 100)'
    )

    args = parser.parse_args()

    result = check_function_sizes(args.crate_path, args.goal, args.hard_limit)

    if result is None:
        return 2

    exceeds_hard_limit, exceeds_goal = result

    if not exceeds_hard_limit and not exceeds_goal:
        print(f"✓ All functions < {args.goal} lines (goal)")
        return 0

    exit_code = 0

    if exceeds_hard_limit:
        print(f"❌ HARD LIMIT VIOLATION: {len(exceeds_hard_limit)} function(s) >= {args.hard_limit} lines:")
        print("   These require explicit approval to ignore.\n")
        for path, line, name, length in sorted(exceeds_hard_limit, key=lambda x: x[3], reverse=True)[:20]:
            print(f"  {path}:{line} {name}() - {length} lines")
        if len(exceeds_hard_limit) > 20:
            print(f"\n  ... and {len(exceeds_hard_limit) - 20} more")
        exit_code = 2
        print()

    if exceeds_goal:
        print(f"{'⚠️ ' if not exceeds_hard_limit else ''}GOAL MISSED: {len(exceeds_goal)} function(s) >= {args.goal} lines but < {args.hard_limit} lines:")
        print("   These should be refactored but don't block progress.\n")
        for path, line, name, length in sorted(exceeds_goal, key=lambda x: x[3], reverse=True)[:20]:
            print(f"  {path}:{line} {name}() - {length} lines (goal: {args.goal}, hard limit: {args.hard_limit})")
        if len(exceeds_goal) > 20:
            print(f"\n  ... and {len(exceeds_goal) - 20} more")
        if exit_code == 0:
            exit_code = 1

    return exit_code


if __name__ == '__main__':
    sys.exit(main())

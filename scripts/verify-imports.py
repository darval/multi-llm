#!/usr/bin/env python3
"""
Verify that all imports are at the top of Rust files.

This script checks that all 'use' statements appear before any non-import code,
following Rust best practices and CLAUDE.md requirements.

Usage:
    python3 scripts/verify-imports.py <crate-path>

Example:
    python3 scripts/verify-imports.py multi-llm

Exit codes:
    0 - All imports are at top
    1 - One or more files have misplaced imports
"""

import os
import sys
import argparse


def is_skippable_line(line):
    """Check if a line should be skipped (empty, comment, attribute)."""
    stripped = line.strip()
    return (not stripped or
            stripped.startswith('//') or
            stripped.startswith('#[') or
            stripped.startswith('#!['))


def is_module_declaration(line):
    """Check if line is a module declaration with opening brace."""
    stripped = line.strip()
    return ((stripped.startswith('pub mod ') or stripped.startswith('mod '))
            and '{' in stripped)


def is_allowed_module_import(line, just_saw_module_decl):
    """Check if this is an allowed 'use super::*' after module declaration."""
    return just_saw_module_decl and line.strip() == 'use super::*;'


def ends_multiline_import(line):
    """Check if line ends a multi-line import."""
    stripped = line.strip()
    return stripped.endswith(';') or stripped == '}' or stripped.endswith('};')


def process_import_line(stripped, state):
    """Process a 'use' import line and update state."""
    # Check if this is an allowed module import
    if is_allowed_module_import(f"{stripped}", state['just_saw_module_decl']):
        state['just_saw_module_decl'] = False
        return None  # No issue

    # Track multi-line imports
    if not stripped.endswith(';'):
        state['in_multiline_import'] = True

    # Check if this import is misplaced
    issue = None
    if (not state['in_imports'] and
        state['first_non_import_line'] > 0 and
        not state['just_saw_module_decl']):
        issue = stripped

    state['just_saw_module_decl'] = False
    return issue


def process_non_import_line(state):
    """Process a non-import line and update state."""
    if state['in_imports']:
        state['in_imports'] = False
        # Store the line number from the calling context
        return True  # Signal to update first_non_import_line
    state['just_saw_module_decl'] = False
    return False


def process_line(line, line_num, state):
    """Process a single line and return any import issue found."""
    if is_skippable_line(line):
        return None

    # Handle multi-line imports
    if state['in_multiline_import']:
        if ends_multiline_import(line):
            state['in_multiline_import'] = False
        return None

    # Handle module declarations
    if is_module_declaration(line):
        state['just_saw_module_decl'] = True
        return None

    stripped = line.strip()

    # Handle imports
    if stripped.startswith('use '):
        issue = process_import_line(stripped, state)
        return (line_num, issue) if issue else None

    # Handle non-import lines
    if process_non_import_line(state):
        state['first_non_import_line'] = line_num
    return None


def check_inline_imports(file_path):
    """Check if a file has imports after non-import code."""
    with open(file_path, 'r') as f:
        lines = f.readlines()

    issues = []
    state = {
        'in_imports': True,
        'in_multiline_import': False,
        'first_non_import_line': 0,
        'just_saw_module_decl': False
    }

    for i, line in enumerate(lines, 1):
        issue = process_line(line, i, state)
        if issue:
            issues.append(issue)

    return issues


def check_imports(crate_path):
    """Check all .rs files in the crate for import placement violations."""
    violations = []
    src_path = os.path.join(crate_path, 'src')

    if not os.path.exists(src_path):
        print(f"Error: {src_path} does not exist", file=sys.stderr)
        return None

    for root, dirs, files in os.walk(src_path):
        # Skip test directories
        if 'tests' in root.split(os.sep):
            continue

        for file in files:
            if not file.endswith('.rs'):
                continue

            path = os.path.join(root, file)
            rel_path = os.path.relpath(path, crate_path)
            issues = check_inline_imports(path)

            if issues:
                violations.append((rel_path, issues))

    return violations


def main():
    parser = argparse.ArgumentParser(
        description='Verify imports are at top of Rust files'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )

    args = parser.parse_args()

    violations = check_imports(args.crate_path)

    if violations is None:
        return 1

    if violations:
        print(f"❌ Found {len(violations)} file(s) with imports after code:\n")
        for path, issues in violations[:10]:
            print(f"  {path}:")
            for line_num, import_stmt in issues[:3]:
                print(f"    Line {line_num}: {import_stmt[:60]}...")
            if len(issues) > 3:
                print(f"    ... and {len(issues) - 3} more imports")
        if len(violations) > 10:
            print(f"\n  ... and {len(violations) - 10} more files")
        return 1
    else:
        print("✓ All imports are at top of files")
        return 0


if __name__ == '__main__':
    sys.exit(main())

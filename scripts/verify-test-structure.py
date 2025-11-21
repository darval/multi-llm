#!/usr/bin/env python3
"""
Verify that unit test structure follows multi-llm conventions.

Checks:
- Test files are in tests/ subdirectories within module hierarchy
- Test files are <600 LOC (hard limit per template)
- Tests have required documentation headers
- Test files mirror source file structure

Usage:
    python3 scripts/verify-test-structure.py <crate-path> [--max-lines LINES]

Example:
    python3 scripts/verify-test-structure.py multi-llm
    python3 scripts/verify-test-structure.py multi-llm --max-lines 600

Exit codes:
    0 - All tests follow structure conventions
    1 - Warnings (should fix)
    2 - Violations (must fix)
"""

import os
import sys
import re
import argparse
from pathlib import Path


def find_test_files(crate_path):
    """Find all test files in the crate."""
    test_files = []
    src_path = Path(crate_path) / 'src'

    if not src_path.exists():
        print(f"Error: {src_path} does not exist", file=sys.stderr)
        return None

    # Find all .rs files in tests/ subdirectories
    for test_file in src_path.rglob('tests/**/*.rs'):
        test_files.append(test_file)

    # Also check for inline #[cfg(test)] modules in large files
    for rs_file in src_path.rglob('*.rs'):
        if '/tests/' not in str(rs_file):
            with open(rs_file, 'r') as f:
                content = f.read()
                if '#[cfg(test)]' in content:
                    test_files.append(rs_file)

    return test_files


def check_file_size(test_file, max_lines):
    """Check if test file exceeds size limit."""
    with open(test_file, 'r') as f:
        line_count = sum(1 for _ in f)

    return line_count, line_count >= max_lines


def check_documentation_header(test_file):
    """Check if test file has required unit documentation."""
    # Skip documentation checks for mod.rs files (they're just module declarations)
    if test_file.name == 'mod.rs':
        return []

    with open(test_file, 'r') as f:
        content = f.read()

    # Look for required documentation patterns
    has_unit_under_test = 'UNIT UNDER TEST:' in content
    has_business_responsibility = 'BUSINESS RESPONSIBILITY:' in content
    has_test_coverage = 'TEST COVERAGE:' in content

    missing = []
    if not has_unit_under_test:
        missing.append('UNIT UNDER TEST')
    if not has_business_responsibility:
        missing.append('BUSINESS RESPONSIBILITY')
    if not has_test_coverage:
        missing.append('TEST COVERAGE')

    return missing


def check_test_structure(test_file, crate_path):
    """Check if test file is in proper location."""
    rel_path = test_file.relative_to(Path(crate_path) / 'src')

    # Check if it's in a tests/ subdirectory
    is_in_tests_dir = 'tests' in rel_path.parts

    # Check if it's inline (has #[cfg(test)] but not in tests/)
    is_inline = False
    if not is_in_tests_dir:
        with open(test_file, 'r') as f:
            if '#[cfg(test)]' in f.read():
                is_inline = True

    return is_in_tests_dir, is_inline


def check_mirrored_structure(crate_path, test_files):
    """Check if test structure mirrors source structure."""
    src_path = Path(crate_path) / 'src'
    issues = []

    # Get all non-test .rs files
    source_files = []
    for rs_file in src_path.rglob('*.rs'):
        if '/tests/' not in str(rs_file) and rs_file.name != 'mod.rs':
            rel_path = rs_file.relative_to(src_path)
            source_files.append(rel_path)

    # For each source file, check if there's a corresponding test
    for source_rel in source_files:
        # Expected test location: replace parent dir with parent/tests/
        parent = source_rel.parent
        expected_test = parent / 'tests' / source_rel.name

        # Check for standard pattern: tests/filename.rs
        test_path = src_path / expected_test

        # Also check for multi-file pattern: tests/filename_*.rs
        stem = source_rel.stem
        test_dir = src_path / parent / 'tests'
        multi_file_tests = []
        if test_dir.exists():
            # Look for files matching filename_*.rs pattern
            multi_file_tests = list(test_dir.glob(f'{stem}_*.rs'))

        # Consider tests exist if either pattern found
        has_tests = test_path.exists() or len(multi_file_tests) > 0

        if not has_tests:
            # Check if it has inline tests
            source_path = src_path / source_rel
            with open(source_path, 'r') as f:
                content = f.read()
                line_count = len(content.splitlines())
                has_inline_tests = '#[cfg(test)]' in content

                # Only report if file is substantial (>100 lines) and has no tests
                if line_count > 100 and not has_inline_tests:
                    issues.append({
                        'source': str(source_rel),
                        'expected_test': str(expected_test),
                        'lines': line_count
                    })

    return issues


def main():
    parser = argparse.ArgumentParser(
        description='Verify unit test structure follows multi-llm conventions'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )
    parser.add_argument(
        '--max-lines',
        type=int,
        default=600,
        help='Maximum lines per test file (default: 600)'
    )

    args = parser.parse_args()

    test_files = find_test_files(args.crate_path)

    if test_files is None:
        return 2

    if not test_files:
        print("⚠️  No test files found")
        return 1

    violations = []
    warnings = []

    print(f"Checking {len(test_files)} test file(s)...\n")

    # Check each test file
    for test_file in test_files:
        rel_path = test_file.relative_to(Path(args.crate_path) / 'src')

        # Check size
        line_count, exceeds_limit = check_file_size(test_file, args.max_lines)
        if exceeds_limit:
            violations.append(f"{rel_path}: {line_count} lines (max: {args.max_lines})")

        # Check documentation
        missing_docs = check_documentation_header(test_file)
        if missing_docs:
            warnings.append(f"{rel_path}: Missing documentation: {', '.join(missing_docs)}")

        # Check structure (location)
        is_in_tests_dir, is_inline = check_test_structure(test_file, args.crate_path)
        if is_inline and line_count > 100:
            warnings.append(f"{rel_path}: Inline tests in large file ({line_count} lines) - should move to tests/ subdirectory")

    # Check mirrored structure
    missing_tests = check_mirrored_structure(args.crate_path, test_files)

    # Report results
    exit_code = 0

    if violations:
        print(f"❌ VIOLATIONS: {len(violations)} test file(s) exceed size limit:")
        print("   These MUST be split (hard limit from template).\n")
        for violation in violations:
            print(f"  {violation}")
        exit_code = 2
        print()

    if warnings:
        print(f"{'⚠️ ' if not violations else ''}WARNINGS: {len(warnings)} issue(s) found:")
        print("   These should be fixed but don't block progress.\n")
        for warning in warnings:
            print(f"  {warning}")
        if exit_code == 0:
            exit_code = 1
        print()

    if missing_tests:
        print(f"{'ℹ️ ' if exit_code == 0 else ''}INFO: {len(missing_tests)} substantial file(s) without tests:")
        print("   Consider adding test coverage.\n")
        for item in missing_tests:  # Show all items
            print(f"  {item['source']} ({item['lines']} lines) -> expected: {item['expected_test']}")
        print()

    if exit_code == 0:
        print("✓ All test files follow structure conventions")

    return exit_code


if __name__ == '__main__':
    sys.exit(main())

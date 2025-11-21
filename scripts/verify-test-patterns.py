#!/usr/bin/env python3
"""
Verify that unit tests follow multi-llm testing patterns.

Checks:
- Required unit documentation (UNIT/UNITS UNDER TEST, BUSINESS RESPONSIBILITY, TEST COVERAGE)
- Proper import patterns (use super::*, use crate::*)
- AAA pattern presence in tests
- Trait compliance tests for multi-implementation traits
- TODO implementation tests exist

Usage:
    python3 scripts/verify-test-patterns.py <crate-path> [--strict]

Example:
    python3 scripts/verify-test-patterns.py multi-llm
    python3 scripts/verify-test-patterns.py multi-llm --strict

Exit codes:
    0 - All tests follow pattern conventions
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
        if test_file.name != 'mod.rs':
            test_files.append(test_file)

    # Also check files with #[cfg(test)]
    for rs_file in src_path.rglob('*.rs'):
        if '/tests/' not in str(rs_file):
            with open(rs_file, 'r') as f:
                if '#[cfg(test)]' in f.read():
                    test_files.append(rs_file)

    return test_files


def check_unit_documentation(file_path):
    """Check if file has required unit documentation."""
    with open(file_path, 'r') as f:
        content = f.read()

    # Only check files that actually have tests
    if not ('#[test]' in content or '#[tokio::test]' in content):
        return {'has_tests': False}

    # Look for required documentation sections
    # Accept both singular "UNIT UNDER TEST:" and plural "UNITS UNDER TEST:"
    has_unit_under_test = bool(re.search(r'UNITS? UNDER TEST:', content))
    has_business_responsibility = bool(re.search(r'BUSINESS RESPONSIBILITY:', content))
    has_test_coverage = bool(re.search(r'TEST COVERAGE:', content))

    missing = []
    if not has_unit_under_test:
        missing.append('UNIT UNDER TEST')
    if not has_business_responsibility:
        missing.append('BUSINESS RESPONSIBILITY')
    if not has_test_coverage:
        missing.append('TEST COVERAGE')

    return {
        'has_tests': True,
        'has_documentation': len(missing) == 0,
        'missing': missing
    }


def check_import_patterns(file_path):
    """Check if imports follow proper patterns."""
    with open(file_path, 'r') as f:
        lines = f.readlines()

    issues = []

    # Check for inline use statements (after function definitions)
    in_function = False
    brace_count = 0

    for i, line in enumerate(lines, 1):
        stripped = line.strip()

        # Track if we're inside a function
        if re.match(r'(pub\s+)?fn\s+\w+', stripped):
            in_function = True
            brace_count = 0

        if in_function:
            brace_count += stripped.count('{') - stripped.count('}')
            if brace_count <= 0:
                in_function = False

        # Check for use statements inside functions
        if in_function and stripped.startswith('use '):
            issues.append({
                'line': i,
                'issue': 'Inline use statement inside function (should be at top of file)',
                'severity': 'violation'
            })

    return issues


def check_aaa_pattern(file_path):
    """Check if tests follow Arrange-Act-Assert pattern."""
    with open(file_path, 'r') as f:
        content = f.read()

    suggestions = []

    # Find all test functions
    test_pattern = r'#\[(tokio::)?test\]\s*(?:async\s+)?fn\s+(\w+)'
    test_matches = list(re.finditer(test_pattern, content))

    for match in test_matches:
        func_name = match.group(2)
        func_start = match.start()

        # Find the function body (rough heuristic)
        func_body_start = content.find('{', func_start)
        if func_body_start == -1:
            continue

        # Find matching closing brace (simplified)
        brace_count = 1
        pos = func_body_start + 1
        while pos < len(content) and brace_count > 0:
            if content[pos] == '{':
                brace_count += 1
            elif content[pos] == '}':
                brace_count -= 1
            pos += 1

        func_body = content[func_body_start:pos]

        # Check for AAA comments
        has_arrange = '// Arrange' in func_body or '//Arrange' in func_body
        has_act = '// Act' in func_body or '//Act' in func_body
        has_assert = '// Assert' in func_body or '//Assert' in func_body

        # Only suggest AAA for longer tests (>10 lines)
        if len(func_body.splitlines()) > 10 and not (has_arrange or has_act or has_assert):
            line_num = content[:func_start].count('\n') + 1
            suggestions.append({
                'line': line_num,
                'function': func_name,
                'issue': 'Consider adding AAA (Arrange-Act-Assert) comments for clarity',
                'severity': 'info'
            })

    return suggestions


def check_trait_compliance(file_path):
    """Check for trait compliance tests when multiple implementations exist."""
    with open(file_path, 'r') as f:
        content = f.read()

    suggestions = []

    # Look for trait definitions
    trait_pattern = r'pub\s+trait\s+(\w+)'
    traits = [m.group(1) for m in re.finditer(trait_pattern, content)]

    if not traits:
        return suggestions

    # Look for multiple implementations of same trait in crate
    # This is a heuristic - we check if trait_compliance_tests module exists
    if 'trait_compliance_tests' not in content and 'trait_compliance' not in content:
        # Check if there are multiple impl blocks
        impl_pattern = r'impl\s+\w+\s+for\s+(\w+)'
        implementations = [m.group(1) for m in re.finditer(impl_pattern, content)]

        if len(implementations) > 1:
            suggestions.append({
                'issue': f'Multiple implementations found ({len(implementations)}) but no trait_compliance_tests module',
                'severity': 'warning',
                'details': 'Consider adding trait compliance tests for consistency'
            })

    return suggestions


def check_todo_tests(file_path):
    """Check if TODO comments in source have corresponding tests."""
    # Get the source file path (remove /tests/ from path)
    src_file = str(file_path).replace('/tests/', '/')

    # If this IS a test file, look for corresponding source
    if '/tests/' in str(file_path):
        # Find the source file
        parts = Path(file_path).parts
        try:
            tests_idx = parts.index('tests')
            src_parts = list(parts[:tests_idx]) + list(parts[tests_idx + 1:])
            src_file = Path(*src_parts)
        except (ValueError, IndexError):
            return []

        if not src_file.exists():
            return []

        with open(src_file, 'r') as f:
            src_content = f.read()

        with open(file_path, 'r') as f:
            test_content = f.read()

        suggestions = []

        # Find TODO comments in source
        todo_pattern = r'//\s*TODO[:\(]([^\\n]+)'
        todos = list(re.finditer(todo_pattern, src_content))

        if todos:
            # Check if test file mentions TODO testing
            has_todo_tests = 'TODO' in test_content and 'test' in test_content.lower()

            if not has_todo_tests and len(todos) > 2:
                suggestions.append({
                    'issue': f'Source file has {len(todos)} TODO comment(s) but no corresponding TODO tests',
                    'severity': 'info',
                    'details': 'Consider adding tests that document TODO behavior'
                })

        return suggestions

    return []


def main():
    parser = argparse.ArgumentParser(
        description='Verify unit tests follow multi-llm testing patterns'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )
    parser.add_argument(
        '--strict',
        action='store_true',
        help='Treat warnings as violations'
    )

    args = parser.parse_args()

    test_files = find_test_files(args.crate_path)

    if test_files is None:
        return 2

    if not test_files:
        print("⚠️  No test files found")
        return 1

    all_violations = []
    all_warnings = []
    all_suggestions = []

    print(f"Checking test patterns in {len(test_files)} test file(s)...\n")

    for test_file in test_files:
        rel_path = test_file.relative_to(Path(args.crate_path) / 'src')

        # Check unit documentation
        doc_check = check_unit_documentation(test_file)
        if doc_check['has_tests'] and not doc_check['has_documentation']:
            all_violations.append({
                'file': str(rel_path),
                'issue': f"Missing required documentation: {', '.join(doc_check['missing'])}",
                'severity': 'violation'
            })

        # Check import patterns
        import_issues = check_import_patterns(test_file)
        for issue in import_issues:
            if issue['severity'] == 'violation':
                all_violations.append({
                    'file': str(rel_path),
                    'line': issue['line'],
                    'issue': issue['issue']
                })

        # Check AAA pattern
        aaa_suggestions = check_aaa_pattern(test_file)
        all_suggestions.extend([{
            'file': str(rel_path),
            **s
        } for s in aaa_suggestions])

        # Check trait compliance
        trait_suggestions = check_trait_compliance(test_file)
        for suggestion in trait_suggestions:
            if suggestion['severity'] == 'warning':
                all_warnings.append({
                    'file': str(rel_path),
                    **suggestion
                })
            else:
                all_suggestions.append({
                    'file': str(rel_path),
                    **suggestion
                })

        # Check TODO tests
        todo_suggestions = check_todo_tests(test_file)
        all_suggestions.extend([{
            'file': str(rel_path),
            **s
        } for s in todo_suggestions])

    # Report results
    exit_code = 0

    if all_violations:
        print(f"❌ VIOLATIONS: {len(all_violations)} pattern violation(s):")
        print("   These MUST be fixed.\n")
        for violation in all_violations[:20]:
            file_info = f"{violation['file']}"
            if 'line' in violation:
                file_info += f":{violation['line']}"
            print(f"  {file_info}")
            print(f"    {violation['issue']}")
        if len(all_violations) > 20:
            print(f"\n  ... and {len(all_violations) - 20} more")
        exit_code = 2
        print()

    if all_warnings or (args.strict and all_suggestions):
        warning_list = all_warnings + (all_suggestions if args.strict else [])
        print(f"{'⚠️ ' if not all_violations else ''}WARNINGS: {len(warning_list)} pattern warning(s):")
        print("   These should be fixed but don't block progress.\n")
        for warning in warning_list[:15]:
            print(f"  {warning['file']}")
            print(f"    {warning['issue']}")
            if 'details' in warning:
                print(f"    {warning['details']}")
        if len(warning_list) > 15:
            print(f"\n  ... and {len(warning_list) - 15} more")
        if exit_code == 0:
            exit_code = 1
        print()

    if all_suggestions and not args.strict:
        print(f"{'ℹ️ ' if exit_code == 0 else ''}INFO: {len(all_suggestions)} suggestion(s):")
        print("   Consider these improvements.\n")
        for suggestion in all_suggestions[:10]:
            print(f"  {suggestion['file']}")
            if 'function' in suggestion:
                print(f"    Function: {suggestion['function']}")
            print(f"    {suggestion['issue']}")
            if 'details' in suggestion:
                print(f"    {suggestion['details']}")
        if len(all_suggestions) > 10:
            print(f"\n  ... and {len(all_suggestions) - 10} more")
        print()

    if exit_code == 0:
        print("✓ All tests follow pattern conventions")

    return exit_code


if __name__ == '__main__':
    sys.exit(main())

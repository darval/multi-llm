#!/usr/bin/env python3
"""
Verify that unit test naming follows multi-llm conventions.

Checks:
- Test functions use test_<unit>_<scenario> pattern
- Helper functions use recognized prefixes:
  - helper_ for generic helper functions
  - create_concrete_<unit>() for units under test
  - create_mock_<dependency>() for mock dependencies
  - setup_ for test setup functions
  - teardown_ for test cleanup functions
  - assert_ for custom assertion helpers
  - verify_ for verification helpers
  - build_ for builder pattern helpers
  - make_ for factory pattern helpers
- Test modules use proper #[cfg(test)] and mod tests patterns
- Trait compliance test naming patterns

Usage:
    python3 scripts/verify-test-naming.py <crate-path>

Example:
    python3 scripts/verify-test-naming.py multi-llm
    python3 scripts/verify-test-naming.py multi-llm

Exit codes:
    0 - All tests follow naming conventions
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


def extract_function_names(file_path):
    """Extract function names from a file."""
    with open(file_path, 'r') as f:
        content = f.read()

    functions = {
        'test_functions': [],
        'helper_functions': [],
        'all_functions': []
    }

    # Find all function definitions
    # Pattern matches: fn name(...) or pub fn name(...)
    pattern = r'(?:pub\s+)?(?:async\s+)?fn\s+(\w+)\s*(?:<[^>]+>)?\s*\('

    for match in re.finditer(pattern, content):
        func_name = match.group(1)
        line_num = content[:match.start()].count('\n') + 1

        # Check if it's preceded by #[test] or #[tokio::test]
        # Look at the immediate preceding lines (not too far back to avoid nested functions)
        lines_before = content[:match.start()].split('\n')

        # Check last few lines for test attribute, but ignore if this is a nested function
        is_test = False
        is_nested = False

        # Look back up to 10 lines
        for i in range(min(10, len(lines_before))):
            line_idx = len(lines_before) - 1 - i
            if line_idx < 0:
                break
            line = lines_before[line_idx].strip()

            # Found a test attribute
            if line.startswith('#[test]') or line.startswith('#[tokio::test]'):
                is_test = True
                break

            # Found a function definition - this is a nested function
            if line.startswith('fn ') or line.startswith('pub fn ') or line.startswith('async fn ') or line.startswith('pub async fn '):
                is_nested = True
                break

        # Nested functions are always helpers, not tests
        if is_nested:
            is_test = False

        if is_test:
            functions['test_functions'].append((func_name, line_num))
        else:
            functions['helper_functions'].append((func_name, line_num))

        functions['all_functions'].append((func_name, line_num, is_test))

    return functions


def check_test_function_naming(test_functions):
    """Check if test functions follow naming conventions."""
    issues = []

    for func_name, line_num in test_functions:
        # Test functions should start with 'test_'
        if not func_name.startswith('test_'):
            issues.append({
                'line': line_num,
                'function': func_name,
                'issue': 'Test function should start with test_',
                'suggestion': f'Rename to test_{func_name}'
            })

    return issues


def is_valid_helper_name(func_name):
    """Check if a function name follows valid helper naming patterns."""
    valid_prefixes = [
        'helper_',           # Generic helper functions
        'create_concrete_',  # Factory for units under test
        'create_mock_',      # Factory for mock dependencies
        'setup_',            # Test setup helpers
        'teardown_',         # Test cleanup helpers
        'assert_',           # Custom assertion helpers
        'verify_',           # Verification helpers
        'build_',            # Builder pattern helpers
        'make_',             # Alternative factory pattern
    ]

    return any(func_name.startswith(prefix) for prefix in valid_prefixes)


def check_helper_function_naming(helper_functions):
    """Check if helper functions follow naming conventions."""
    issues = []
    suggestions = []

    for func_name, line_num in helper_functions:
        # Skip standard helper patterns
        if is_valid_helper_name(func_name):
            # Check for proper patterns within valid prefixes
            if func_name.startswith('create_'):
                # Should be either create_concrete_ or create_mock_
                if not func_name.startswith('create_concrete_') and not func_name.startswith('create_mock_'):
                    suggestions.append({
                        'line': line_num,
                        'function': func_name,
                        'issue': 'create_ helpers should be create_concrete_<unit>() or create_mock_<dependency>()',
                        'severity': 'warning'
                    })
        else:
            # Helper function doesn't follow any recognized pattern - might be a test missing test_ prefix
            # But only flag if it looks like a test (common test keywords)
            test_keywords = ['should', 'when', 'verifies', 'validates', 'checks', 'example', 'caller']
            if any(keyword in func_name.lower() for keyword in test_keywords):
                suggestions.append({
                    'line': line_num,
                    'function': func_name,
                    'issue': 'Helper function should use a recognized prefix (helper_, create_mock_, create_concrete_, setup_, etc.)',
                    'severity': 'warning'
                })

    return issues, suggestions


def check_module_structure(file_path):
    """Check if file has proper #[cfg(test)] module structure."""
    with open(file_path, 'r') as f:
        content = f.read()

    issues = []

    # Check for #[cfg(test)]
    has_cfg_test = '#[cfg(test)]' in content

    # Check for mod tests
    has_mod_tests = re.search(r'mod\s+tests\s*\{', content) is not None

    # If file has tests, it should have proper structure
    if '#[test]' in content or '#[tokio::test]' in content:
        if '/tests/' not in str(file_path):
            # Inline tests should have #[cfg(test)] mod tests
            if not has_cfg_test:
                issues.append({
                    'issue': 'File has tests but missing #[cfg(test)]',
                    'severity': 'violation'
                })
            if not has_mod_tests:
                issues.append({
                    'issue': 'File has tests but missing mod tests block',
                    'severity': 'violation'
                })

    return issues


def check_trait_compliance_tests(file_path):
    """Check for trait compliance test patterns."""
    with open(file_path, 'r') as f:
        content = f.read()

    suggestions = []

    # Look for traits that might need compliance tests
    # Check if file has multiple implementations of same trait
    trait_impl_pattern = r'impl\s+(\w+)\s+for\s+(\w+)'
    trait_impls = {}

    for match in re.finditer(trait_impl_pattern, content):
        trait_name = match.group(1)
        impl_name = match.group(2)

        if trait_name not in trait_impls:
            trait_impls[trait_name] = []
        trait_impls[trait_name].append(impl_name)

    # Check if compliance tests exist for multi-implementation traits
    for trait_name, impl_names in trait_impls.items():
        if len(impl_names) > 1:
            # Look for trait_compliance_tests module
            if 'trait_compliance_tests' not in content:
                suggestions.append({
                    'trait': trait_name,
                    'implementations': impl_names,
                    'issue': f'Multiple implementations of {trait_name} found but no trait_compliance_tests module',
                    'severity': 'info'
                })

    return suggestions


def main():
    parser = argparse.ArgumentParser(
        description='Verify unit test naming follows multi-llm conventions'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
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

    print(f"Checking naming conventions in {len(test_files)} test file(s)...\n")

    for test_file in test_files:
        rel_path = test_file.relative_to(Path(args.crate_path) / 'src')

        # Extract functions
        functions = extract_function_names(test_file)

        # Check test function naming
        test_naming_issues = check_test_function_naming(functions['test_functions'])
        for issue in test_naming_issues:
            all_violations.append({
                'file': str(rel_path),
                **issue
            })

        # Check helper function naming
        helper_issues, helper_suggestions = check_helper_function_naming(functions['helper_functions'])
        for issue in helper_issues:
            all_violations.append({
                'file': str(rel_path),
                **issue
            })
        for suggestion in helper_suggestions:
            all_warnings.append({
                'file': str(rel_path),
                **suggestion
            })

        # Check module structure
        module_issues = check_module_structure(test_file)
        for issue in module_issues:
            if issue['severity'] == 'violation':
                all_violations.append({
                    'file': str(rel_path),
                    **issue
                })
            else:
                all_warnings.append({
                    'file': str(rel_path),
                    **issue
                })

        # Check trait compliance
        trait_suggestions = check_trait_compliance_tests(test_file)
        for suggestion in trait_suggestions:
            all_suggestions.append({
                'file': str(rel_path),
                **suggestion
            })

    # Report results
    exit_code = 0

    if all_violations:
        print(f"❌ VIOLATIONS: {len(all_violations)} naming convention violation(s):")
        print("   These MUST be fixed.\n")
        for violation in all_violations[:20]:
            file_info = f"{violation['file']}:{violation.get('line', '?')}"
            func_info = f"{violation.get('function', 'N/A')}"
            print(f"  {file_info} - {func_info}")
            print(f"    Issue: {violation['issue']}")
            if 'suggestion' in violation:
                print(f"    Suggestion: {violation['suggestion']}")
        if len(all_violations) > 20:
            print(f"\n  ... and {len(all_violations) - 20} more")
        exit_code = 2
        print()

    if all_warnings:
        print(f"{'⚠️ ' if not all_violations else ''}WARNINGS: {len(all_warnings)} naming convention warning(s):")
        print("   These should be fixed but don't block progress.\n")
        for warning in all_warnings[:15]:
            file_info = f"{warning['file']}:{warning.get('line', '?')}"
            func_info = f"{warning.get('function', 'N/A')}"
            print(f"  {file_info} - {func_info}")
            print(f"    {warning['issue']}")
        if len(all_warnings) > 15:
            print(f"\n  ... and {len(all_warnings) - 15} more")
        if exit_code == 0:
            exit_code = 1
        print()

    if all_suggestions:
        print(f"{'ℹ️ ' if exit_code == 0 else ''}INFO: {len(all_suggestions)} suggestion(s):")
        print("   Consider these improvements.\n")
        for suggestion in all_suggestions[:10]:
            print(f"  {suggestion['file']}")
            print(f"    {suggestion['issue']}")
        if len(all_suggestions) > 10:
            print(f"\n  ... and {len(all_suggestions) - 10} more")
        print()

    if exit_code == 0:
        print("✓ All tests follow naming conventions")

    return exit_code


if __name__ == '__main__':
    sys.exit(main())

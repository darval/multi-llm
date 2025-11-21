#!/usr/bin/env python3
"""
Verify that Rust files follow naming conventions from docs/guides/file-naming-conventions.md.

This script checks for:
- Mock files in src/ instead of tests/
- Non-snake_case file names
- Compliance with reserved names

Usage:
    python3 scripts/verify-file-naming.py <crate-path>

Example:
    python3 scripts/verify-file-naming.py multi-llm

Exit codes:
    0 - All files follow conventions
    1 - One or more files violate conventions
"""

import os
import sys
import re
import argparse


# Reserved file names that are expected
RESERVED_NAMES = {'mod.rs', 'lib.rs', 'main.rs', 'error.rs', 'config.rs'}


def check_file_naming(crate_path):
    """Check all .rs files for naming convention violations."""
    issues = []
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

            # Skip reserved names
            if file in RESERVED_NAMES:
                continue

            # Check if mock file is in src/ instead of tests/
            if '_mock.rs' in file and '/tests/' not in path:
                issues.append((rel_path, "Mock file should be in tests/ directory"))

            # Check for snake_case (basic validation)
            base_name = file.replace('.rs', '')
            if not re.match(r'^[a-z][a-z0-9_]*$', base_name):
                issues.append((rel_path, "File name should be snake_case"))

    return issues


def main():
    parser = argparse.ArgumentParser(
        description='Verify Rust file naming conventions'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )

    args = parser.parse_args()

    issues = check_file_naming(args.crate_path)

    if issues is None:
        return 1

    if issues:
        print(f"❌ Found {len(issues)} file naming issue(s):\n")
        for path, msg in issues[:20]:
            print(f"  {path}")
            print(f"    → {msg}")
        if len(issues) > 20:
            print(f"\n  ... and {len(issues) - 20} more issues")
        return 1
    else:
        print("✓ All files follow naming conventions")
        return 0


if __name__ == '__main__':
    sys.exit(main())

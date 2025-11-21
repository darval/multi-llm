#!/usr/bin/env python3
"""
Verify that type names follow conventions from docs/guides/type-naming-conventions.md.

This script checks for:
- Overly generic type names (Helper, Manager, Data, etc.)
- Unnecessary 'Core' prefix on domain entities
- Unnecessary 'multi-llm' prefix (except multi-llmError)

Usage:
    python3 scripts/verify-type-naming.py <crate-path>

Example:
    python3 scripts/verify-type-naming.py multi-llm

Exit codes:
    0 - All types follow conventions
    1 - One or more types violate conventions
"""

import os
import sys
import re
import argparse


# Generic names to flag
GENERIC_NAMES = {'Helper', 'Manager', 'Data', 'Utils', 'Common', 'Base'}


def check_type_naming(crate_path):
    """Check all .rs files for type naming convention violations."""
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

            with open(path, 'r') as f:
                content = f.read()

            # Find struct/enum/trait definitions
            type_defs = re.findall(r'pub\s+(struct|enum|trait)\s+(\w+)', content)

            for type_kind, type_name in type_defs:
                # Check for overly generic names
                if type_name in GENERIC_NAMES:
                    issues.append((rel_path, type_kind, type_name,
                                 "overly generic name - use descriptive, domain-specific name"))

                # Check for Core prefix on domain entities
                if type_name.startswith('Core'):
                    issues.append((rel_path, type_kind, type_name,
                                 "avoid 'Core' prefix - use domain-specific naming"))

                # Check for multi-llm prefix (except multi-llmError)
                if type_name.startswith('multi-llm') and type_name != 'multi-llmError':
                    issues.append((rel_path, type_kind, type_name,
                                 "avoid 'multi-llm' prefix - context is clear from crate"))

    return issues


def main():
    parser = argparse.ArgumentParser(
        description='Verify Rust type naming conventions'
    )
    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )

    args = parser.parse_args()

    issues = check_type_naming(args.crate_path)

    if issues is None:
        return 1

    if issues:
        print(f"❌ Found {len(issues)} type naming issue(s):\n")
        for path, kind, name, msg in issues[:20]:
            print(f"  {path}:")
            print(f"    {kind} {name} - {msg}")
        if len(issues) > 20:
            print(f"\n  ... and {len(issues) - 20} more issues")
        return 1
    else:
        print("✓ All type names follow conventions")
        return 0


if __name__ == '__main__':
    sys.exit(main())

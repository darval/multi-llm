#!/usr/bin/env python3
"""
Verify code coverage for multi-llm unit tests.

Uses cargo-llvm-cov to measure code coverage and enforces minimum thresholds.

Requirements:
- cargo-llvm-cov must be installed: cargo install cargo-llvm-cov

Usage:
    python3 scripts/verify-test-coverage.py <crate-path> [options]

Examples:
    # Check coverage for entire crate (unit tests only)
    python3 scripts/verify-test-coverage.py multi-llm

    # Check coverage including integration tests
    python3 scripts/verify-test-coverage.py multi-llm --include-integration

    # Check coverage for specific module (runs only tests matching filter)
    python3 scripts/verify-test-coverage.py multi-llm --test-filter domain::

    # Custom thresholds
    python3 scripts/verify-test-coverage.py multi-llm --goal 90 --minimum 80

    # Generate HTML report
    python3 scripts/verify-test-coverage.py multi-llm --html

    # Show per-file coverage details (filtered to specified crate only)
    # Shows clean paths (e.g., context/types.rs) and crate-specific statistics
    python3 scripts/verify-test-coverage.py multi-llm --show-files

    # Show per-file coverage for all crates (including dependencies)
    # Shows full paths (e.g., multi-llm/src/domain/user.rs) and overall statistics
    python3 scripts/verify-test-coverage.py multi-llm --show-all-files

    # Include integration tests with detailed file view
    python3 scripts/verify-test-coverage.py multi-llm --include-integration --show-files

Exit codes:
    0 - Coverage meets goal threshold
    1 - Coverage meets minimum but not goal (warning)
    2 - Coverage below minimum threshold (violation)
    3 - Error running coverage tool
"""

import sys
import os
import re
import argparse
import subprocess
from pathlib import Path
from typing import Optional, Dict, List


def check_llvm_cov_installed() -> bool:
    """Check if cargo-llvm-cov is installed."""
    try:
        result = subprocess.run(
            ['cargo', 'llvm-cov', '--version'],
            capture_output=True,
            text=True,
            timeout=10
        )
        return result.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


def parse_coverage_line(line: str) -> Optional[Dict[str, float]]:
    """Parse a single coverage summary line from llvm-cov output."""
    # Look for the TOTAL line which has format:
    # TOTAL    7309    6818    6.72%    831    740    10.95%    6540    6025    7.87%    0    0    -
    if 'TOTAL' in line:
        parts = line.split()
        if len(parts) >= 13:
            try:
                # Extract coverage percentages
                region_cov = float(parts[3].rstrip('%'))
                function_cov = float(parts[6].rstrip('%'))
                line_cov = float(parts[9].rstrip('%'))

                return {
                    'region': region_cov,
                    'function': function_cov,
                    'line': line_cov,
                }
            except (ValueError, IndexError):
                pass
    return None


def parse_file_coverage(lines: List[str]) -> Dict[str, Dict[str, float]]:
    """Parse per-file coverage from llvm-cov output."""
    files = {}

    for line in lines:
        # Skip header and total lines
        if line.startswith('Filename') or line.startswith('TOTAL') or line.startswith('-'):
            continue

        parts = line.split()
        if len(parts) >= 13:
            try:
                filename = parts[0]
                line_cov = float(parts[9].rstrip('%'))

                files[filename] = {
                    'line': line_cov,
                }
            except (ValueError, IndexError):
                pass

    return files


def parse_uncovered_lines(lines: List[str]) -> Dict[str, List[int]]:
    """Parse uncovered lines section from llvm-cov --show-missing-lines output.

    Returns a dict mapping file paths to lists of uncovered line numbers.
    Example output format:
        Uncovered Lines:
        /Users/rick/git/multi-llm/multi-llm/src/coordinator/agent.rs: 56, 57, 58, 78-82
    """
    uncovered = {}
    in_uncovered_section = False

    for line in lines:
        if 'Uncovered Lines:' in line:
            in_uncovered_section = True
            continue

        if not in_uncovered_section:
            continue

        # Parse lines like: "/path/to/file.rs: 56, 57, 78-82, 100"
        if ':' in line:
            parts = line.split(':', 1)
            if len(parts) == 2:
                filepath = parts[0].strip()
                line_nums_str = parts[1].strip()

                # Parse line numbers (handles both single numbers and ranges)
                line_nums = []
                for part in line_nums_str.split(','):
                    part = part.strip()
                    if '-' in part:
                        # Range like "78-82"
                        try:
                            start, end = part.split('-')
                            line_nums.extend(range(int(start), int(end) + 1))
                        except ValueError:
                            pass
                    else:
                        # Single line number
                        try:
                            line_nums.append(int(part))
                        except ValueError:
                            pass

                if line_nums:
                    uncovered[filepath] = line_nums

    return uncovered


def run_coverage(crate_path: Path, test_filter: Optional[str], html: bool, show_files: bool, include_integration: bool, show_uncovered: bool, timeout: int) -> Optional[str]:
    """Run cargo-llvm-cov and return the output."""
    if not crate_path.exists():
        print(f"Error: Crate path {crate_path} does not exist", file=sys.stderr)
        return None

    cmd = [
        'cargo', 'llvm-cov',
        '--package', crate_path.name,
        '--lib',
    ]

    # Add integration tests if requested
    if include_integration:
        cmd.append('--tests')

    if html:
        cmd.append('--html')
    elif show_uncovered:
        # Show uncovered lines
        cmd.append('--show-missing-lines')
    elif not show_files:
        # Only use summary-only if we don't need per-file details
        cmd.append('--summary-only')

    # Add test filter if specified
    if test_filter:
        cmd.extend(['--', test_filter])
    elif include_integration:
        # For integration tests, serialize execution to avoid rate limits
        cmd.extend(['--', '--test-threads=1'])

    print(f"Running coverage analysis for {crate_path.name}...")
    if include_integration:
        print("  Test types: unit + integration")
    else:
        print("  Test types: unit only")
    if test_filter:
        print(f"  Test filter: {test_filter}")
    print()

    try:
        # Set environment for integration tests to reduce LLM token usage
        test_env = os.environ.copy()
        if include_integration:
            test_env['MULTI_LLM_TEST_MODE'] = '1000'  # Reduce context tokens for integration tests

        result = subprocess.run(
            cmd,
            cwd=crate_path.parent,
            capture_output=True,
            text=True,
            timeout=timeout,
            env=test_env
        )

        # Check if stderr contains actual errors (not just warnings)
        # cargo-llvm-cov may output warnings to stderr even on success
        if result.returncode != 0:
            # Only fail if returncode is non-zero AND we have actual error output
            stderr_lower = result.stderr.lower()
            has_fatal_error = any(marker in stderr_lower for marker in [
                'error: could not compile',
                'error: failed to',
                'error: no such',
                'could not compile',
            ])

            if has_fatal_error:
                print("Error running coverage analysis:", file=sys.stderr)
                print(result.stderr, file=sys.stderr)
                return None
            # If returncode is non-zero but no fatal errors, treat as warning
            elif result.stderr:
                print("Warning from cargo-llvm-cov:", file=sys.stderr)
                print(result.stderr, file=sys.stderr)

        # Return stdout even if there were warnings in stderr
        return result.stdout

    except subprocess.TimeoutExpired:
        print(f"Error: Coverage analysis timed out after {timeout} seconds", file=sys.stderr)
        return None
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return None


def print_coverage_report(coverage: Dict[str, float], goal: float, minimum: float, test_filter: Optional[str], filtered_cov: Optional[float] = None, crate_name: Optional[str] = None):
    """Print formatted coverage report."""
    # Use filtered coverage if available, otherwise use overall
    line_cov = filtered_cov if filtered_cov is not None else coverage['line']

    status = determine_status(line_cov, goal, minimum)

    print("\n" + "="*60)
    print("CODE COVERAGE REPORT")
    print("="*60)

    if test_filter:
        print(f"Test Filter: {test_filter}")

    if crate_name and filtered_cov is not None:
        print(f"\n{crate_name} Line Coverage: {line_cov:.2f}%")
        print(f"Overall Line Coverage:     {coverage['line']:.2f}%")
    elif filtered_cov is not None:
        print(f"\nFiltered Line Coverage: {line_cov:.2f}%")
        print(f"Overall Line Coverage:  {coverage['line']:.2f}%")
    else:
        print(f"\nLine Coverage:     {line_cov:.2f}%")

    # Only show function/region coverage for overall stats (they're not per-file)
    if filtered_cov is None:
        print(f"Function Coverage: {coverage['function']:.2f}%")
        print(f"Region Coverage:   {coverage['region']:.2f}%")

    print(f"\nStatus: {status}\n")

    print_thresholds(goal, minimum)
    print_status_details(line_cov, goal, minimum)

    print("="*60 + "\n")


def determine_status(coverage: float, goal: float, minimum: float) -> str:
    """Determine status based on coverage and thresholds."""
    if coverage >= goal:
        return "✅ EXCELLENT"
    if coverage >= minimum:
        return "⚠️  WARNING"
    return "❌ VIOLATION"


def print_thresholds(goal: float, minimum: float):
    """Print threshold information."""
    print("Thresholds:")
    print(f"  Goal:    {goal:.0f}% (target for excellent coverage)")
    print(f"  Minimum: {minimum:.0f}% (hard limit)")


def print_status_details(coverage: float, goal: float, minimum: float):
    """Print detailed status information."""
    if coverage >= goal:
        print("\n✓ Coverage meets goal threshold!")
    elif coverage >= minimum:
        gap = goal - coverage
        print("\n⚠ Coverage meets minimum but below goal")
        print(f"  Need {gap:.2f}% more to reach goal")
    else:
        min_gap = minimum - coverage
        goal_gap = goal - coverage
        print("\n✗ Coverage below minimum threshold")
        print(f"  Need {min_gap:.2f}% more to meet minimum")
        print(f"  Need {goal_gap:.2f}% more to reach goal")


def calculate_average_coverage(files: Dict[str, Dict[str, float]], filter_prefix: str) -> Optional[float]:
    """Calculate average line coverage for files matching filter."""
    filtered = {k: v for k, v in files.items() if filter_prefix in k}

    if not filtered:
        return None

    total = sum(v['line'] for v in filtered.values())
    return total / len(filtered)


def calculate_filtered_coverage_stats(files: Dict[str, Dict[str, float]], crate_name: str) -> Optional[float]:
    """Calculate average line coverage for files in the specified crate.

    Handles two cases:
    1. Files with crate prefix (e.g., 'multi-llm/src/...')
    2. Files without prefix (relative paths from crate root, e.g., 'domain/story.rs')
    """
    # Check if any files have the crate prefix
    has_crate_prefix = any(k.startswith(f"{crate_name}/") for k in files.keys())

    if has_crate_prefix:
        # Filter by crate prefix (multi-crate scenario)
        crate_files = {k: v for k, v in files.items() if k.startswith(f"{crate_name}/")}
    else:
        # No prefix means all files are from this crate (single-crate scenario)
        crate_files = files

    if not crate_files:
        return None

    total = sum(v['line'] for v in crate_files.values())
    return total / len(crate_files)


def extract_module_prefix(test_filter: Optional[str]) -> Optional[str]:
    """Extract module prefix from test filter (e.g., 'billing::' -> 'billing/')."""
    if not test_filter:
        return None

    # Remove trailing :: if present
    module = test_filter.rstrip(':')

    # Convert to path format
    return f"{module}/"


def format_filename_with_prefix(filename: str, strip_prefix: Optional[str]) -> str:
    """Strip crate prefix from filename if filtering to single crate."""
    if strip_prefix and filename.startswith(strip_prefix):
        return filename[len(strip_prefix):]
    return filename


def format_uncovered_lines(line_numbers: List[int], max_display: int = None) -> str:
    """Format uncovered line numbers into a compact string.

    Converts consecutive lines into ranges (e.g., [1,2,3,5,6] -> "1-3, 5-6")
    Optionally truncates if there are too many lines.
    """
    if not line_numbers:
        return ""

    sorted_lines = sorted(line_numbers)
    ranges = []
    start = sorted_lines[0]
    end = sorted_lines[0]

    for num in sorted_lines[1:]:
        if num == end + 1:
            end = num
        else:
            if start == end:
                ranges.append(str(start))
            else:
                ranges.append(f"{start}-{end}")
            start = num
            end = num

    # Add the last range
    if start == end:
        ranges.append(str(start))
    else:
        ranges.append(f"{start}-{end}")

    result = ", ".join(ranges)
    if max_display and len(result) > max_display:
        return result[:max_display] + f"... ({len(sorted_lines)} total lines)"
    return result


def find_uncovered_for_file(filepath: str, uncovered_map: Dict[str, List[int]]) -> Optional[List[int]]:
    """Find uncovered lines for a file, matching by suffix if exact match fails."""
    # Try exact match first
    if filepath in uncovered_map:
        return uncovered_map[filepath]

    # Try matching by suffix (file path without full absolute path)
    for uncov_path, lines in uncovered_map.items():
        if uncov_path.endswith(filepath) or filepath in uncov_path:
            return lines

    return None


def print_file_coverage(files: Dict[str, Dict[str, float]], show_all: bool, module_filter: Optional[str] = None, crate_name: Optional[str] = None, uncovered_lines: Optional[Dict[str, List[int]]] = None):
    """Print per-file coverage details.

    Args:
        files: Dictionary of filename -> coverage data
        show_all: If True, show all files; if False, filter to crate_name
        module_filter: Optional module prefix to filter by (e.g., 'billing/')
        crate_name: Name of the crate being analyzed (e.g., 'multi-llm')
        uncovered_lines: Optional dict mapping file paths to uncovered line numbers
    """
    print("\nPer-File Coverage:")
    print("-" * 60)

    # Track whether we should strip the crate prefix from display
    strip_prefix = None

    # Filter files to only show the crate we're analyzing (unless show_all is True)
    if not show_all and crate_name:
        # Check if any files have the crate prefix
        has_crate_prefix = any(k.startswith(f"{crate_name}/") for k in files.keys())

        if has_crate_prefix:
            # Filter by crate prefix (multi-crate scenario)
            files = {k: v for k, v in files.items() if k.startswith(f"{crate_name}/")}
            # Strip the prefix when displaying since we're filtering to just this crate
            strip_prefix = f"{crate_name}/src/"

    if module_filter:
        # Show only files matching the module filter
        module_files = {k: v for k, v in files.items() if module_filter in k}

        if module_files:
            module_name = module_filter.rstrip('/').capitalize()
            print(f"\n{module_name} Files:")
            for filename, cov in sorted(module_files.items()):
                line_cov = cov['line']
                status = "✅" if line_cov >= 90 else "⚠️ " if line_cov >= 80 else "❌"
                display_name = format_filename_with_prefix(filename, strip_prefix)
                print(f"  {status} {display_name:50s} {line_cov:6.2f}%")

                # Show uncovered lines if available
                if uncovered_lines:
                    uncov = find_uncovered_for_file(filename, uncovered_lines)
                    if uncov:
                        formatted_lines = format_uncovered_lines(uncov)
                        print(f"       Uncovered: {formatted_lines}")

            # Calculate average
            avg = calculate_average_coverage(files, module_filter)
            if avg:
                print(f"\n  Average coverage: {avg:.2f}%")
        else:
            print(f"\n  No files found matching '{module_filter}'")

    if show_all or not module_filter:
        # Show all files grouped by module
        shown_files = set()

        # Common module patterns
        common_modules = ['billing/', 'domain/', 'executor/', 'llm/', 'storage/', 'tools/', 'agents/']

        for prefix in common_modules:
            module_files = {k: v for k, v in files.items() if prefix in k and k not in shown_files}

            if module_files:
                module_name = prefix.rstrip('/').capitalize()
                print(f"\n{module_name} Files:")
                for filename, cov in sorted(module_files.items()):
                    line_cov = cov['line']
                    status = "✅" if line_cov >= 90 else "⚠️ " if line_cov >= 80 else "❌"
                    display_name = format_filename_with_prefix(filename, strip_prefix)
                    print(f"  {status} {display_name:50s} {line_cov:6.2f}%")

                    # Show uncovered lines if available
                    if uncovered_lines:
                        uncov = find_uncovered_for_file(filename, uncovered_lines)
                        if uncov:
                            formatted_lines = format_uncovered_lines(uncov)
                            print(f"       Uncovered: {formatted_lines}")

                    shown_files.add(filename)

        # Show any remaining uncategorized files
        other_files = {k: v for k, v in files.items() if k not in shown_files}
        if other_files:
            print("\nOther Files:")
            for filename, cov in sorted(other_files.items()):
                line_cov = cov['line']
                status = "✅" if line_cov >= 90 else "⚠️ " if line_cov >= 80 else "❌"
                display_name = format_filename_with_prefix(filename, strip_prefix)
                print(f"  {status} {display_name:50s} {line_cov:6.2f}%")

                # Show uncovered lines if available
                if uncovered_lines:
                    uncov = find_uncovered_for_file(filename, uncovered_lines)
                    if uncov:
                        formatted_lines = format_uncovered_lines(uncov)
                        print(f"       Uncovered: {formatted_lines}")

    print("-" * 60)


def main():
    """Main entry point."""
    parser = create_argument_parser()
    args = parser.parse_args()

    if not validate_arguments(args):
        return 2

    if not check_llvm_cov_installed():
        print("Error: cargo-llvm-cov is not installed", file=sys.stderr)
        print("Install it with: cargo install cargo-llvm-cov", file=sys.stderr)
        return 3

    crate_path = Path(args.crate_path).resolve()
    show_uncovered = args.show_uncovered
    # Auto-enable show_files when show_uncovered is requested
    show_files = args.show_files or args.show_all_files or show_uncovered
    output = run_coverage(crate_path, args.test_filter, args.html, show_files, args.include_integration, show_uncovered, args.timeout)

    if output is None:
        return 3

    lines = output.splitlines()

    # Parse overall coverage
    coverage = None
    for line in lines:
        coverage = parse_coverage_line(line)
        if coverage:
            break

    if coverage is None:
        print("Error: Failed to parse coverage data", file=sys.stderr)
        print("\nOutput from coverage tool:")
        print(output)
        return 3

    # Parse per-file coverage
    files = parse_file_coverage(lines)

    # Parse uncovered lines if show_uncovered was requested
    uncovered = None
    if show_uncovered:
        uncovered = parse_uncovered_lines(lines)

    # Calculate filtered coverage if test filter is specified
    filtered_cov = None
    module_filter = None
    crate_filter_name = None

    if args.test_filter and files:
        # Extract module name from filter using helper function
        module_filter = extract_module_prefix(args.test_filter)
        if module_filter:
            filtered_cov = calculate_average_coverage(files, module_filter)
    elif show_files and not args.show_all_files and files:
        # When showing only crate files (not all files), calculate crate-specific stats
        filtered_cov = calculate_filtered_coverage_stats(files, crate_path.name)
        crate_filter_name = crate_path.name

    # Print report
    print_coverage_report(coverage, args.goal, args.minimum, args.test_filter, filtered_cov, crate_filter_name)

    # Print per-file coverage if requested or if using filter
    if show_files or args.test_filter:
        if files:
            print_file_coverage(files, args.show_all_files, module_filter, crate_path.name, uncovered)

    # Print HTML report location if generated
    if args.html:
        html_file = crate_path.parent / 'target' / 'llvm-cov' / 'html' / 'index.html'
        if html_file.exists():
            print(f"\nHTML report: {html_file}\n")

    # Determine exit code based on filtered coverage if available, otherwise overall
    line_cov = filtered_cov if filtered_cov is not None else coverage['line']
    if line_cov >= args.goal:
        return 0  # Excellent
    if line_cov >= args.minimum:
        return 1  # Warning
    return 2  # Violation


def create_argument_parser():
    """Create and configure argument parser."""
    parser = argparse.ArgumentParser(
        description='Verify code coverage for multi-llm unit tests',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s multi-llm
  %(prog)s multi-llm --test-filter domain::
  %(prog)s multi-llm --goal 90 --minimum 80 --html
  %(prog)s multi-llm --show-files
        """
    )

    parser.add_argument(
        'crate_path',
        help='Path to the crate to check (e.g., multi-llm)'
    )

    parser.add_argument(
        '--test-filter',
        help='Test filter (e.g., domain::, session::)'
    )

    parser.add_argument(
        '--goal',
        type=float,
        default=90.0,
        help='Goal coverage percentage (default: 90.0)'
    )

    parser.add_argument(
        '--minimum',
        type=float,
        default=80.0,
        help='Minimum acceptable coverage percentage (default: 80.0)'
    )

    parser.add_argument(
        '--html',
        action='store_true',
        help='Generate HTML coverage report'
    )

    parser.add_argument(
        '--show-files',
        action='store_true',
        help='Show per-file coverage details (filtered to the specified crate)'
    )

    parser.add_argument(
        '--show-all-files',
        action='store_true',
        help='Show all files from all crates (not just the specified crate)'
    )

    parser.add_argument(
        '--show-uncovered',
        action='store_true',
        help='Show uncovered lines for each file (detailed line-by-line view)'
    )

    parser.add_argument(
        '--include-integration',
        action='store_true',
        help='Include integration tests (tests/) in coverage analysis'
    )

    parser.add_argument(
        '--timeout',
        type=int,
        default=300,
        help='Timeout in seconds (default: 300)'
    )

    return parser


def validate_arguments(args) -> bool:
    """Validate command-line arguments."""
    if args.goal < args.minimum:
        print("Error: Goal threshold must be >= minimum threshold", file=sys.stderr)
        return False
    return True


if __name__ == '__main__':
    sys.exit(main())

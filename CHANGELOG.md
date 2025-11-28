# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2025-11-28

### Added

- Add GitHub mirror workflow (Fix #12)
- Add release.sh script for automated release workflow
- Add comprehensive design documentation and architecture decision records

### Fixed

- Fix doctests to use unwrap_response! macro for feature compatibility
- Fix #10: Remove stale DualLLMConfig doctest from config module
- Fix #11: Add Gitea Action for CI testing on development branch
- Fix #10: Remove domain-specific DualLLMConfig and LLMPath types
- Fix #8: Create runnable examples with unwrap_response! macro
- Fix #7: Add comprehensive rustdoc comments to all public API types
- Fix #6: Ensure consistent use of log_*! macros
- Fix #5: Restructure module hierarchy
- Fix #4: Narrow public API surface to ~28 types
- Fix #3: Implement extended (1-hour) cache support with accurate pricing
- Fix #2: Feature-gate events system
- Fix #1: Remove legacy naming (Executor*, MyStoryError, AgentContext)

### Changed

- Update documentation for Phase 2 completion and Phase 3 planning
- Improve Rust API Guidelines compliance
- Progress on #14: Improve test coverage (82.59% -> 84.15%)

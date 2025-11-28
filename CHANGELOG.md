# Changelog

All notable changes to this project will be documented in this file.

## [0.2.1] - 2025-11-28

### Added

- GitHub mirror workflow for public release pipeline
- Automated release script with semver analysis

### Fixed

- CI workflow compatibility with Gitea Actions runner

## [0.2.0] - 2025-11-28

Initial public release.

### Features

- **Multi-Provider Support**: OpenAI, Anthropic, Ollama, LM Studio
- **Unified Message Format**: Provider-agnostic message architecture
- **Prompt Caching**: Native support for Anthropic's 5-minute and 1-hour caching
- **Tool Calling**: First-class function/tool calling support across providers
- **Async-First**: Built on Tokio for high-performance async I/O
- **Type-Safe**: Leverage Rust's type system to catch errors at compile time
- **Optional Events**: Feature-gated business event logging for observability

### Documentation

- Comprehensive design documentation
- Architecture decision records (ADRs)
- Runnable examples with `unwrap_response!` macro

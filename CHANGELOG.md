# Changelog

All notable changes to this project will be documented in this file.


## [1.0.0] - 2025-11-28
## [1.0.0] - 2025-11-28

First stable release.

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
- 9 runnable examples covering all major use cases
- Full rustdoc coverage

### Infrastructure

- Automated release pipeline (Gitea CI → GitHub mirror → crates.io)
- GitHub Actions for publishing and release creation

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/2.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-08-27

### Added

- A newline-delimited JSON-RPC STDIO MCP server that exposes GuardGen 2.3.0
  through the `generate_include_guard` tool.
- Configuration for macro prefixes and suffixes, C linkage wrappers, line
  endings, and UUID v7 or v4 generation.
- MCP invalid-parameter responses for unknown fields and malformed argument
  values.
- Session-scoped GuardGen state that preserves monotonic UUID v7 generation
  across successive calls.
- Cargo installation, build, and MCP client setup documentation.

[Unreleased]: https://github.com/daisuke-nagao/guardgen_mcp/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/daisuke-nagao/guardgen_mcp/releases/tag/v1.0.0

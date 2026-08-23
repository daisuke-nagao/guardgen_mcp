# GuardGen MCP

This crate exposes GuardGen 2.3.0 as a single MCP tool over newline-delimited
JSON-RPC STDIO. It writes protocol messages to stdout only; diagnostics belong
on stderr.

## Build and run

```text
cargo build --release --locked
target/release/guardgen_mcp
```

Configure an MCP client to launch the built executable with no arguments. The
server keeps one GuardGen generator alive for the session, so successive UUID
v7 calls preserve GuardGen's monotonic generation state.

## Tool

The only advertised tool is `generate_include_guard`. Its optional arguments
are:

| Argument | Values/default | Description |
| --- | --- | --- |
| `prefix` | `UUID` | Non-empty ASCII C identifier |
| `suffix` | `null` | Non-empty ASCII alphanumeric/underscore segment when present |
| `language` | `none` | `none`, `c`, or `cxx` |
| `line_ending` | `none` | `none`, `lf`, or `crlf` |
| `uuid_version` | `v7` | `v7` or `v4` |

The result is the generated include-guard source as text content. Unknown
arguments and invalid values return an MCP invalid-params error. File output
options such as `output` and `overwrite` are not supported.

## Checks

```text
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

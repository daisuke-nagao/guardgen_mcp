# GuardGen MCP

This crate exposes GuardGen 2.3.0 as a single MCP tool over newline-delimited
JSON-RPC STDIO. It writes protocol messages to stdout only; diagnostics belong
on stderr.

## Install

```text
cargo install --locked --git https://github.com/daisuke-nagao/guardgen_mcp.git
```

This installs the `guardgen_mcp` binary into Cargo's bin directory
(`~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows). If `cargo install`
added that directory to `PATH` for you, `guardgen_mcp` is now on your PATH and
an MCP client can launch it by name. If it isn't found, add that directory to
`PATH` yourself (or point the MCP client at the full path printed by `cargo
install`).

## Build and run

```text
cargo build --release --locked
target/release/guardgen_mcp
```

Configure an MCP client to launch the built executable with no arguments. The
server keeps one GuardGen generator alive for the session, so successive UUID
v7 calls preserve GuardGen's monotonic generation state.

## Release archives

Each GitHub Release archive contains these four files at its root:

- `guardgen_mcp` (`guardgen_mcp.exe` on Windows)
- `THIRD-PARTY-LICENSES.html`
- `LICENSE-MIT`
- `LICENSE-APACHE`

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

## Development

Create a repository-local virtual environment, install the development
dependency, and install the Git hook:

```text
uv venv
uv pip install --requirement requirements-dev.txt
uv run pre-commit install
```

The first hook run downloads isolated Node and Go environments. System-wide
Node and Go installations are not required.

Run every commit check against the repository:

```text
uv run pre-commit run --all-files
```

Update hook revisions when needed:

```text
uv run pre-commit autoupdate
```

Run the test suite separately:

```text
cargo test --locked
```

Generate the target-specific third-party license report after a release build:

```text
cargo install --locked --features cli --version 0.9.2 cargo-about
cargo build --release --locked --target <target>
cargo about generate --locked --fail --target <target> --output-file target/<target>/release/THIRD-PARTY-LICENSES.html about.hbs
```

The generated HTML remains under `target` and is not committed.

## License

Licensed under either of the following licenses, at your option:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

SPDX license expression: `MIT OR Apache-2.0`.

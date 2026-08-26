# GuardGen MCP

This crate exposes GuardGen 2.3.0 as a single MCP tool over newline-delimited
JSON-RPC STDIO. It writes protocol messages to stdout only; diagnostics belong
on stderr.

## Install

```text
cargo install --locked --git https://github.com/daisuke-nagao/guardgen_mcp.git --tag v1.0.0
```

This installs the `guardgen_mcp` binary into Cargo's bin directory
(`~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows). If `cargo install`
added that directory to `PATH` for you, `guardgen_mcp` is now on your PATH and
an MCP client can launch it by name. If it isn't found, add that directory to
`PATH` yourself or configure the MCP client with the full path to the
executable.

## Configure MCP clients

GuardGen MCP uses STDIO and requires no command-line arguments. The examples
below assume that `guardgen_mcp` is available on `PATH`.

### Claude Code

Register GuardGen MCP in the user scope so that it is available across
projects:

```text
claude mcp add --scope user --transport stdio guardgen_mcp -- guardgen_mcp
```

Verify the configuration with:

```text
claude mcp get guardgen_mcp
```

The `--scope user` option stores the MCP server in Claude Code's user
configuration. To use a project-specific or local configuration instead, use
the corresponding Claude Code MCP scope.

### Codex

Register GuardGen MCP with Codex:

```text
codex mcp add guardgen_mcp -- guardgen_mcp
```

Verify the configuration with:

```text
codex mcp list
```

By default, Codex stores the MCP configuration in `~/.codex/config.toml`.
The configuration is shared by Codex clients that use this configuration,
including the Codex CLI and IDE extension.

If `guardgen_mcp` is not on `PATH`, replace the final `guardgen_mcp` in either
registration command with the full path to the executable.

## Build and run

Build from source with:

```text
cargo build --release --locked
```

Run the resulting executable directly with:

```text
target/release/guardgen_mcp
```

On Windows, the executable is:

```text
target\release\guardgen_mcp.exe
```

To configure an MCP client to use a locally built binary, use its full path as
the STDIO server command instead of `guardgen_mcp`.

The server keeps one GuardGen generator alive for the MCP session, so
successive UUID v7 calls preserve GuardGen's monotonic generation state.

## Release archives

Each GitHub Release archive contains these four files at its root:

- `guardgen_mcp` (`guardgen_mcp.exe` on Windows)
- `THIRD-PARTY-LICENSES.html`
- `LICENSE-MIT`
- `LICENSE-APACHE`

After extracting an archive, either add the directory containing
`guardgen_mcp` to `PATH` or configure the MCP client with the full path to the
executable.

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

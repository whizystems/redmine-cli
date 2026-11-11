# Change: Add MCP Server Mode

## Why
MCP clients without a shell (Claude Desktop / Cowork) cannot run `rdm` directly. An MCP server lets them use the same Redmine functionality.

## What Changes
- New `rdm mcp` subcommand: MCP server over stdio (JSON-RPC 2.0, newline-delimited).
- Tools auto-generated from the clap command tree: one tool per leaf subcommand (e.g. `issue_list`, `time_create`), flags become JSON Schema properties.
- Tool calls are converted to argv and dispatched through the CLI code path (no duplicated logic).
- `profile *` and `config` are not exposed; credentials resolve exactly like the CLI.
- Optional `format` argument (`markdown` default, `json`).

## Impact
- Affected specs: new `mcp-server` capability
- Affected code: `src/mcp.rs` (new), `src/main.rs`, `src/cli/mod.rs`, `Cargo.toml` (tokio `io-std`, `io-util`)

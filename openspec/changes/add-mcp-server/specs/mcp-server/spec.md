## ADDED Requirements

### Requirement: MCP Server Mode
The CLI SHALL provide an `rdm mcp` subcommand that runs a Model Context Protocol server over stdio, using the same credential resolution as other commands.

#### Scenario: Initialize handshake
- **WHEN** a client sends `initialize`
- **THEN** the server responds with server info and a `tools` capability

#### Scenario: Notifications are not answered
- **WHEN** a client sends a message without an `id`
- **THEN** the server sends no response

### Requirement: Tools Mirror CLI Commands
The server SHALL expose one tool per leaf CLI subcommand, named by joining the command path with `_`, excluding `profile`, `config` and `mcp`.

#### Scenario: Tool listing
- **WHEN** a client calls `tools/list`
- **THEN** tools such as `issue_list`, `issue_relation_add` and `time_create` are returned with JSON Schema inputs derived from the CLI flags

#### Scenario: Tool call parity
- **WHEN** a client calls `issue_list` with `{"assigned_to": "me", "limit": 5}`
- **THEN** the result text equals the output of `rdm issue list --assigned-to me --limit 5`

#### Scenario: Tool errors
- **WHEN** a tool call fails validation or the Redmine API returns an error
- **THEN** the result has `isError: true` and the formatted error as text

# rdm - Redmine CLI

An agent-first Redmine CLI with markdown-optimized output.

## Features

- **Markdown-first output** - Default format optimized for LLM/agent consumption
- **JSON envelope format** - Structured output for programmatic pipelines with `--format json`
- **Full CRUD operations** - Issues, time entries, and projects
- **Multiple profiles** - Manage credentials for different Redmine instances
- **Activity caching** - 24-hour TTL cache for time entry activities
- **Standardized exit codes** - Predictable error handling for scripts
- **Cross-platform binaries** - macOS, Linux, and Windows
- **Installable agent skill** - Includes a reusable `redmine-cli-workflows` skill for agent-driven Redmine tasks

## Installation

### macOS / Linux (Homebrew)

```bash
brew tap whizystems/tap
brew install rdm
```

### Windows (Scoop)

```powershell
scoop bucket add whizystems https://github.com/whizystems/scoop-bucket
scoop install rdm
```

### Manual Download

Download the latest release from [GitHub Releases](https://github.com/whizystems/redmine-cli/releases):

```bash
# macOS (Apple Silicon)
curl -LO https://github.com/whizystems/redmine-cli/releases/latest/download/rdm-aarch64-apple-darwin.tar.gz
tar -xzf rdm-aarch64-apple-darwin.tar.gz
sudo mv rdm /usr/local/bin/

# macOS (Intel)
curl -LO https://github.com/whizystems/redmine-cli/releases/latest/download/rdm-x86_64-apple-darwin.tar.gz
tar -xzf rdm-x86_64-apple-darwin.tar.gz
sudo mv rdm /usr/local/bin/

# Linux
curl -LO https://github.com/whizystems/redmine-cli/releases/latest/download/rdm-x86_64-unknown-linux-gnu.tar.gz
tar -xzf rdm-x86_64-unknown-linux-gnu.tar.gz
sudo mv rdm /usr/local/bin/

# Linux (no sudo — user-scoped install)
# ~/.local/bin is on PATH by default on most modern distributions
mkdir -p ~/.local/bin
mv rdm ~/.local/bin/
```

### From Source

```bash
# Clone the repository
git clone https://github.com/whizystems/redmine-cli.git
cd redmine-cli

# Install with cargo
cargo install --path .
```

## Agent Skill

This repository also includes the `redmine-cli-workflows` skill for agent environments that support installable skills.

Install it with:

```bash
npx skills add https://github.com/whizystems/redmine-cli/ --skill redmine-cli-workflows
```

The skill helps agents choose the right `rdm` commands for common Redmine workflows such as connectivity checks, issue lookup and updates, time logging, reporting, and custom-field handling.

## Quick Start

### 1. Set up credentials

**Option A: Environment variables**

```bash
export REDMINE_URL=https://redmine.example.com
export REDMINE_API_KEY=your-api-key
```

**Option B: Create a profile**

```bash
rdm profile add --name work --url https://redmine.example.com --api-key your-api-key
```

### 2. Test the connection

```bash
rdm ping
```

### 3. View your user info

```bash
rdm me
```

### 4. List your open issues

```bash
rdm issue list --assigned-to me --status open
```

### 5. Log time

```bash
rdm time create --issue 123 --hours 2.5 --activity Development --comment "Implemented feature X"
```

## Commands Reference

### General

| Command | Description |
|---------|-------------|
| `rdm ping` | Check connection and authentication |
| `rdm me` | Show current user information |
| `rdm config` | Show current configuration |

### Profile Management

| Command | Description |
|---------|-------------|
| `rdm profile add` | Add a new profile |
| `rdm profile use <name>` | Set the active profile |
| `rdm profile list` | List all profiles |
| `rdm profile delete` | Delete a profile |

### Projects

| Command | Description |
|---------|-------------|
| `rdm project list` | List projects |
| `rdm project get` | Get project details |

### Issues

| Command | Description |
|---------|-------------|
| `rdm issue list` | List issues with filters |
| `rdm issue get` | Get issue details |
| `rdm issue create` | Create a new issue |
| `rdm issue update` | Update an existing issue |
| `rdm issue relation list --id <id>` | List an issue's relations |
| `rdm issue relation add --id <id> --to <id> --type <type> [--delay <days>]` | Add a relation |
| `rdm issue relation remove --relation-id <id>` | Remove a relation |

**Issue list filters:**
- `--project <id>` - Filter by project
- `--status <open|closed|*|id>` - Filter by status
- `--assigned-to <me|id>` - Filter by assignee
- `--author <me|id>` - Filter by author
- `--tracker <id>` - Filter by tracker
- `--version <id|name>` - Filter by target version (repeatable or comma-separated; names resolve within `--project`)
- `--subject <text>` - Filter by exact subject match
- `--search <text>` - Search issues by text (subject/description)
- `--query-id <id>` - Run a saved query (see `rdm query list`); add `--project` for a project-scoped query
- `--cf <id>=<value>` - Filter by custom field (repeatable)
- `--include-relations` - Embed each issue's `relations` in the output

### Relations and parent issues

`rdm issue get` includes the issue's `parent` (`{id}`, when set) and `relations` (`[{id, issue_id, issue_to_id, relation_type, delay}]`, Redmine's shape). `issue list` always carries `parent`; add `--include-relations` for `relations`.

Relation types: `relates`, `duplicates`, `duplicated`, `blocks`, `blocked`, `precedes`, `follows`, `copied_to`, `copied_from`. `--delay` applies to `precedes`/`follows`.

Set a parent with `rdm issue create --parent <id>` or `rdm issue update --id <id> --parent <id>`; clear it with `--parent none`.
Unassign an issue with `rdm issue update --id <id> --assigned-to none`.

### Saved Queries

| Command | Description |
|---------|-------------|
| `rdm query list` | List saved issue queries |
| `rdm query list --type time` | List saved time-entry queries |

Run a saved query via `--query-id` on `issue list` or `time list`. Project-scoped queries also require `--project <id>` (Redmine resolves the query against the project in the URL path).

### Versions

| Command | Description |
|---------|-------------|
| `rdm version list --project <id>` | List a project's versions (target versions / milestones), including shared ones |

Set an issue's target version with `rdm issue update --id <id> --version <id|name>` (names resolve within the issue's project).

### Time Entries

| Command | Description |
|---------|-------------|
| `rdm time list` | List time entries |
| `rdm time get` | Get time entry details |
| `rdm time create` | Create a time entry |
| `rdm time update` | Update a time entry |
| `rdm time delete` | Delete a time entry |
| `rdm time activities list` | List available activities |

**Time list filters:**
- `--project <id>` - Filter by project
- `--issue <id>` - Filter by issue
- `--user <me|id>` - Filter by user
- `--from <YYYY-MM-DD>` - Filter from date
- `--to <YYYY-MM-DD>` - Filter to date
- `--query-id <id>` - Run a saved time-entry query (see `rdm query list --type time`); add `--project` for a project-scoped query
- `--cf <id>=<value>` - Filter by custom field (repeatable)
- `--group-by <field>` - Group results by: `user`, `project`, `activity`, `issue`, `spent_on`, or `cf_<id>`

### Users

| Command | Description |
|---------|-------------|
| `rdm user list` | List users |

**User list filters:**
- `--status <active|registered|locked>` - Filter by status

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `REDMINE_URL` | Redmine server URL |
| `REDMINE_API_KEY` | Your Redmine API key |

### Profiles

Profiles are stored in the configuration file and allow managing multiple Redmine instances:

```bash
# Add profiles
rdm profile add --name work --url https://work.redmine.com --api-key key1
rdm profile add --name personal --url https://personal.redmine.com --api-key key2

# Switch between profiles
rdm profile use personal

# List profiles
rdm profile list
```

### Configuration Precedence

1. CLI flags (`--url`, `--api-key`)
2. Environment variables (`REDMINE_URL`, `REDMINE_API_KEY`)
3. Config file (active profile)

## Output Formats

### Markdown (default)

The default output format is markdown, optimized for readability and LLM consumption:

```bash
$ rdm issue get --id 123
```

```markdown
## Issue #123

| Field | Value |
|-------|-------|
| Subject | Fix login bug |
| Project | Backend |
| Status | In Progress |
| Priority | High |
| Assignee | John Doe |
```

### JSON (`--format json`)

Use the `--format json` flag for structured output:

```bash
$ rdm issue get --id 123 --format json
```

```json
{
  "ok": true,
  "data": {
    "id": 123,
    "subject": "Fix login bug",
    "project": { "id": 1, "name": "Backend" },
    "status": { "id": 2, "name": "In Progress" },
    "priority": { "id": 3, "name": "High" }
  },
  "meta": {}
}
```

For list operations, the envelope includes pagination metadata:

```json
{
  "ok": true,
  "data": { "issues": [...] },
  "meta": {
    "total_count": 150,
    "limit": 25,
    "offset": 0,
    "next_offset": 25
  }
}
```

Error responses follow the same envelope structure:

```json
{
  "ok": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Not found: Issue #999"
  },
  "meta": {}
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Validation/argument error |
| 3 | Authentication/configuration error |
| 4 | Resource not found |
| 5 | API/server/network error |

## Examples

### List my open issues assigned to me

```bash
rdm issue list --assigned-to me --status open
```

### List issues in a specific project

```bash
rdm issue list --project backend --status "*"
```

### Create an issue

```bash
rdm issue create \
  --project 1 \
  --subject "Implement user authentication" \
  --description "Add OAuth2 support" \
  --tracker 1 \
  --priority 2 \
  --assigned-to 5
```

### Create an issue with custom fields

```bash
rdm issue create \
  --project 1 \
  --subject "High priority task" \
  --cf 5=urgent \
  --cf 6=backend
```

### Update an issue

```bash
rdm issue update --id 123 --status 3 --done-ratio 50 --notes "Halfway done"

# Change tracker (must be enabled for the issue's project)
rdm issue update --id 123 --tracker 2
```

### Update an issue with custom fields

```bash
rdm issue update --id 123 --cf 5=normal --cf 7="Q2 2024"
```

### Log time to an issue

```bash
rdm time create --issue 123 --hours 2.5 --activity Development
```

### Log time to a project (no issue)

```bash
rdm time create --project 1 --hours 1.0 --activity Meeting --comment "Sprint planning"
```

### List time entries for a date range

```bash
rdm time list --user me --from 2024-01-01 --to 2024-01-31
```

### List time entries grouped by project

```bash
rdm time list --user me --from 2024-01-01 --to 2024-01-31 --group-by project
```

### Search issues by text

```bash
rdm issue list --search "authentication error" --project backend
```

### Run a saved query

```bash
# List available queries
rdm query list
rdm query list --type time

# Run a global saved query
rdm issue list --query-id 40

# Run a project-scoped saved query (requires --project)
rdm issue list --query-id 5 --project 10
rdm time list --query-id 112 --project 113
```

### Filter issues by target version

```bash
# Find version IDs and names
rdm version list --project backend

# By name or ID (repeatable or comma-separated)
rdm issue list --project backend --version "Milestone 1,Milestone 2" --status '*'
rdm issue list --version 116 --version 117

# Move an issue to a version
rdm issue update --id 123 --version "Milestone 2"
```

### Filter issues by custom field

```bash
rdm issue list --project backend --cf 5=urgent --cf 6=backend
```

### List users

```bash
rdm user list --status active
```

### Use in shell scripts

```bash
#!/bin/bash
set -e

# Check connection
rdm ping --format json | jq -e '.ok' > /dev/null

# Get issue and extract status
STATUS=$(rdm issue get --id 123 --format json | jq -r '.data.status.name')
echo "Issue status: $STATUS"
```

### Dry run mode

Preview what would be sent without executing:

```bash
rdm issue create --project 1 --subject "Test" --dry-run
```

### Debug mode

Enable debug logging to stderr:

```bash
rdm --debug issue list
```

## Building from Source

### Prerequisites

- Rust 1.70 or later

### Build

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

The binary will be at `target/release/rdm` (or `target/release/rdm.exe` on Windows).

### Run Tests

```bash
cargo test
```

### Cross-compilation Targets

The project is configured for:

- `x86_64-unknown-linux-musl` - Linux static binary
- `x86_64-pc-windows-msvc` - Windows binary

## License

MIT

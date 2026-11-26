# cli-core Specification

## Purpose
TBD - created by archiving change add-redmine-agent-cli. Update Purpose after archive.
## Requirements
### Requirement: Markdown-First Output Format
The CLI SHALL default to well-formatted Markdown output optimized for LLM/agent consumption. The output SHALL:
- Use clear section headers (`##`, `###`) to organize information
- Present tabular data in Markdown tables with aligned columns
- Include contextual metadata (IDs, timestamps, counts) that agents need for follow-up actions
- Use consistent formatting patterns across all commands
- Provide actionable information (e.g., "Use `rdm issue get --id 123` for details")

#### Scenario: List command outputs Markdown table
- **WHEN** `rdm issue list --project myproject` is executed
- **THEN** the output is a Markdown table with columns for ID, Subject, Status, Assignee, and Updated

#### Scenario: Get command outputs structured Markdown
- **WHEN** `rdm issue get --id 123` is executed
- **THEN** the output includes a header with issue ID and subject, followed by labeled fields in a readable format

#### Scenario: Success message includes context
- **WHEN** `rdm time create --issue 123 --hours 2 --activity Development` succeeds
- **THEN** the output confirms creation with the new entry's ID and key details the agent can reference

### Requirement: Output Format Selection
The CLI SHALL support `--format` flag with options:
- `markdown` (default): Human and agent-readable structured Markdown
- `json`: Machine-parseable JSON envelope for programmatic pipelines

#### Scenario: Default format is Markdown
- **WHEN** a command is executed without `--format`
- **THEN** the output is formatted as Markdown

#### Scenario: JSON format returns envelope
- **WHEN** a command is executed with `--format json`
- **THEN** the output is a JSON object with `ok`, `data`, `meta`, and `error` fields

### Requirement: JSON Output Envelope
When `--format json` is specified, the CLI SHALL return responses in a consistent JSON envelope format containing:
- `ok`: boolean indicating success/failure
- `data`: the response payload (object, array, or null)
- `meta`: metadata object (pagination info, timestamps)
- `error`: error details object (null on success)

#### Scenario: Successful JSON command returns envelope with data
- **WHEN** a command executes successfully with `--format json`
- **THEN** the response has `ok: true`, populated `data`, and `error: null`

#### Scenario: Failed JSON command returns envelope with error
- **WHEN** a command fails with `--format json`
- **THEN** the response has `ok: false`, `data: null`, and populated `error` with code and message

### Requirement: Error Output for Agents
Error messages SHALL be formatted to help agents understand and recover:
- Clear error type/code at the start
- Concise description of what went wrong
- Suggestion for resolution when applicable
- In Markdown mode: formatted as a blockquote or admonition
- In JSON mode: structured error object with code, message, and details

#### Scenario: Validation error suggests fix
- **WHEN** `rdm time create --hours -1` is executed (invalid hours)
- **THEN** the error explains that hours must be positive and shows correct usage

#### Scenario: Not found error includes ID
- **WHEN** `rdm issue get --id 99999` is executed for non-existent issue
- **THEN** the error clearly states "Issue #99999 not found"

### Requirement: Exit Code Contract
The CLI SHALL return standardized exit codes:
- 0: Success
- 2: Validation/argument errors
- 3: Authentication/configuration errors
- 4: Resource not found
- 5: API/server/network errors

#### Scenario: Successful execution returns exit code 0
- **WHEN** a command completes successfully
- **THEN** the process exits with code 0

#### Scenario: Invalid arguments return exit code 2
- **WHEN** required arguments are missing or invalid
- **THEN** the process exits with code 2

#### Scenario: Missing credentials return exit code 3
- **WHEN** API key or URL is not configured
- **THEN** the process exits with code 3

### Requirement: Non-Interactive Default
The CLI SHALL NOT prompt for input unless the `--interactive` flag is provided.

#### Scenario: Missing required argument fails without prompt
- **WHEN** a required argument is missing and `--interactive` is not set
- **THEN** the CLI returns an error without prompting

### Requirement: Configuration Precedence
The CLI SHALL load configuration in this order (first wins):
1. CLI flags (`--url`, `--api-key`)
2. Environment variables (`REDMINE_URL`, `REDMINE_API_KEY`)
3. Active profile from config file
4. Error if credentials not found

#### Scenario: CLI flags override environment variables
- **WHEN** both `--url` flag and `REDMINE_URL` env var are set
- **THEN** the CLI uses the value from the flag

#### Scenario: Environment variables override config file
- **WHEN** `REDMINE_URL` env var is set and config file has different URL
- **THEN** the CLI uses the value from the environment variable

### Requirement: Profile Management
The CLI SHALL support multiple named profiles stored in a config file with commands:
- `profile add --name <name> --url <url> --api-key <key>`
- `profile use <name>` - set active profile
- `profile list` - list all profiles
- `profile delete --name <name>`
- `config show` - display current configuration (redacting API key)

#### Scenario: Adding a new profile
- **WHEN** `rdm profile add --name work --url https://redmine.example.com --api-key xxx` is executed
- **THEN** a new profile named "work" is saved to the config file

#### Scenario: Switching active profile
- **WHEN** `rdm profile use work` is executed
- **THEN** the "work" profile becomes the active profile for subsequent commands

### Requirement: Diagnostic Commands
The CLI SHALL provide diagnostic commands:
- `ping`: Verify connectivity and authentication
- `me`: Return the authenticated user's information

#### Scenario: Ping with valid credentials
- **WHEN** `rdm ping` is executed with valid credentials
- **THEN** the response indicates successful connection with server info

#### Scenario: Me returns current user
- **WHEN** `rdm me` is executed
- **THEN** the response contains the authenticated user's id, login, and name

### Requirement: Project Commands
The CLI SHALL provide project commands:
- `project list [--limit N --offset N]`
- `project get --id <id> | --identifier <identifier>`

#### Scenario: List projects with pagination
- **WHEN** `rdm project list --limit 10 --offset 0` is executed
- **THEN** the response contains up to 10 projects in a Markdown table with pagination info

#### Scenario: Get project by identifier
- **WHEN** `rdm project get --identifier my-project` is executed
- **THEN** the response contains the project details in structured Markdown

### Requirement: Issue Commands
The CLI SHALL provide issue commands:
- `issue list` with filters: `--project`, `--status`, `--assigned-to`, `--author`, `--limit`, `--offset`
- `issue get --id <id>`
- `issue create --project <id> --subject <text>` with optional: `--description`, `--assigned-to`, `--priority`, `--status`, `--tracker`, `--cf <key=value>`
- `issue update --id <id>` with optional: `--subject`, `--description`, `--status`, `--assigned-to`, `--priority`, `--notes`, `--cf <key=value>`

#### Scenario: List issues with filters
- **WHEN** `rdm issue list --project myproject --status open --assigned-to me` is executed
- **THEN** the response contains issues matching all filter criteria in a Markdown table

#### Scenario: Create issue with required fields
- **WHEN** `rdm issue create --project myproject --subject "New bug"` is executed
- **THEN** a new issue is created and the response confirms with issue ID and details

#### Scenario: Update issue with notes
- **WHEN** `rdm issue update --id 123 --status closed --notes "Fixed in commit abc"` is executed
- **THEN** the issue is updated and the response confirms the changes

### Requirement: Time Entry Activity Caching
The CLI SHALL cache time entry activities to disk with a 24-hour TTL. The `time activities list` command refreshes the cache.

#### Scenario: Activities cached on first fetch
- **WHEN** `rdm time activities list` is executed and no cache exists
- **THEN** activities are fetched from API and saved to cache

#### Scenario: Activities served from cache within TTL
- **WHEN** `rdm time activities list` is executed within 24 hours of last fetch
- **THEN** activities are returned from cache without API call

### Requirement: Time Entry Commands
The CLI SHALL provide full CRUD for time entries:
- `time create --hours <float> --activity <name|id>` with one of `--issue <id>` or `--project <id>`, optional: `--spent-on`, `--comment`, `--user`
- `time list` with filters: `--issue`, `--project`, `--user`, `--from`, `--to`, `--limit`, `--offset`
- `time get --id <id>`
- `time update --id <id>` with optional: `--hours`, `--spent-on`, `--activity`, `--comment`
- `time delete --id <id>`

#### Scenario: Create time entry with activity name
- **WHEN** `rdm time create --issue 123 --hours 2.5 --activity Development --comment "Code review"` is executed
- **THEN** a time entry is created and the response confirms with entry ID and summary

#### Scenario: Create time entry defaults to today
- **WHEN** `rdm time create --issue 123 --hours 1 --activity Design` is executed without `--spent-on`
- **THEN** the time entry is created with today's date

#### Scenario: List time entries with date range
- **WHEN** `rdm time list --user me --from 2024-01-01 --to 2024-01-31` is executed
- **THEN** the response contains time entries in a Markdown table with totals

#### Scenario: Delete time entry
- **WHEN** `rdm time delete --id 456` is executed
- **THEN** the time entry is deleted and success is confirmed with entry ID

### Requirement: Debug and Dry-Run Modes
The CLI SHALL support:
- `--debug`: Write request/response metadata to stderr
- `--dry-run`: Print the HTTP request without executing it

#### Scenario: Debug mode logs to stderr
- **WHEN** any command is executed with `--debug`
- **THEN** HTTP request and response details are written to stderr

#### Scenario: Dry-run shows request without executing
- **WHEN** `rdm time create --issue 123 --hours 1 --activity Dev --dry-run` is executed
- **THEN** the request method, URL, and body are printed but no HTTP call is made
- **AND** the command exits with code 0 (a completed dry run is not an error)

#### Scenario: Dry-run in JSON format
- **WHEN** a write command is executed with `--dry-run --format json`
- **THEN** a success envelope is printed with `data.dry_run: true`, `data.method`, `data.path`, and `data.body` (omitted when the request has no body)

### Requirement: Cross-Platform Distribution
The CLI SHALL be distributed as static binaries:
- Windows: `x86_64-pc-windows-msvc`
- Linux: `x86_64-unknown-linux-musl` with rustls TLS

#### Scenario: Linux binary runs without dependencies
- **WHEN** the Linux musl binary is executed on a minimal container
- **THEN** it runs without requiring OpenSSL or other shared libraries


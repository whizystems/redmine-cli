---
name: redmine-cli-workflows
description: Use this skill when a task requires operational access to Redmine through the local `rdm` CLI, such as checking connectivity, reviewing or updating issues, logging or analyzing time entries, resolving project or user IDs, or working with Redmine custom fields, even if the user does not explicitly mention Redmine or the CLI.
---

# Redmine CLI Workflows

Use `rdm` when the user needs Redmine operations and the local CLI is available.

## Overview

Default approach:

1. Verify `rdm` exists and auth is usable.
2. Discover IDs and valid values before mutating.
3. Use `--format json` when the result will drive follow-up decisions.
4. Prefer readback after mutations when confirmation matters.

Fast checks:

```bash
rdm --version
rdm --help
rdm ping
rdm me
```

On PowerShell, if the binary may be missing:

```powershell
Get-Command rdm
```

If `rdm` is unavailable, stop and say so unless the user explicitly wants install help.

## When to Use

Use this skill when the user needs any of these:

- confirm Redmine connectivity or auth
- inspect current user or active config
- list, get, create, or update issues
- filter or set an issue's target version (milestone), or list a project's versions
- read, add, or remove issue relations (blocks, precedes, relates, ...), or set/clear a parent issue
- list, get, create, update, delete, or group time entries
- run saved (custom) queries for issues or time entries
- resolve project or user IDs before a write
- work with custom-field filters or values

Do not rely on this skill alone when:

- the local binary behavior differs from the skill
- the user asks for a flag or subcommand not covered here
- exact JSON field names matter and you have not inspected live output yet

Then verify with local help:

```bash
rdm --help
rdm issue --help
rdm time --help
```

## Quick Reference

Core command groups:

- General: `ping`, `me`, `config`
- Profiles: `profile add`, `profile use`, `profile list`, `profile delete`
- Projects: `project list`, `project get`
- Issues: `issue list`, `issue get`, `issue create`, `issue update`
- Relations: `issue relation list`, `issue relation add`, `issue relation remove`
- Time: `time list`, `time get`, `time create`, `time update`, `time delete`, `time activities list`
- Queries: `query list` (add `--type time` for time-entry queries)
- Versions: `version list --project <id>`
- Users: `user list`, `user me`

Configuration precedence:

1. `--url`, `--api-key`
2. `REDMINE_URL`, `REDMINE_API_KEY`
3. active profile in config

Use markdown by default for human summaries. Use JSON for selection, branching, pagination, or pipelines.

```bash
rdm issue get --id 123
rdm issue get --id 123 --format json
```

JSON responses use `ok`, `data`, and `meta`. Errors use `error`.

## Common Workflows

### Verify setup

```bash
rdm ping
rdm me
rdm config
```

### Configure a profile

```bash
rdm profile add --name work --url https://redmine.example.com --api-key your-api-key
rdm profile use work
rdm ping
```

### Find projects and users

```bash
rdm project list
rdm project get --id 1
rdm project get --identifier backend

rdm user list --status active
rdm user me
```

### Find issues

Exact or filter-based issue lookup:

```bash
rdm issue list --assigned-to me --status open
rdm issue list --project backend --subject "Authentication error"
rdm issue list --project backend --cf 5=urgent --cf 6=backend
```

Text search:

```bash
rdm issue list --project backend --search "authentication error"
```

Important: `--search` uses the search endpoint. In the current CLI it composes with `--project`, `--limit`, and `--offset`, but not the other issue-list filters. Do not assume `--status`, `--assigned-to`, `--author`, `--tracker`, `--subject`, or `--cf` still apply when `--search` is present.

### Run a saved query

Saved (custom) queries live in Redmine's UI. List them, then run one via `--query-id`:

```bash
rdm query list
rdm query list --type time

rdm issue list --query-id 40
rdm time list --query-id 112 --project 113
```

Important: a project-scoped query (scope shown as `project <id>` in `query list`) only resolves when you also pass `--project <id>`; without it the API returns 404. Global queries (scope `global`) run without `--project`. Query type is fixed at save time: use `--query-id` on `issue list` for issue queries and on `time list` for time-entry queries.

### Work with target versions (milestones)

```bash
rdm version list --project backend
rdm issue list --project backend --version "Milestone 1,Milestone 2" --status '*'
rdm issue update --id 123 --version "Milestone 2"
```

Issues expose their target version as `fixed_version` (`{id, name}`) in JSON. `--version` accepts IDs or names, repeatable or comma-separated. Names on `issue list` resolve within `--project` (required for names); on `issue update` they resolve within the issue's own project. IDs work anywhere.

### Work with relations and parent issues

```bash
rdm issue relation list --id 123
rdm issue relation add --id 123 --to 124 --type blocks
rdm issue relation add --id 123 --to 125 --type precedes --delay 2
rdm issue relation remove --relation-id 9
rdm issue update --id 125 --parent 123
rdm issue update --id 125 --parent none
rdm issue update --id 125 --assigned-to none   # unassign
rdm issue list --project backend --include-relations --format json
```

`issue get` JSON includes `parent` (`{id}`, omitted when none) and `relations` (`[{id, issue_id, issue_to_id, relation_type, delay}]`). A relation is stored once: when `issue_to_id` is the issue you asked about, read it inversely (`blocks` from #120 to #123 means #123 is blocked by #120). Remove relations by relation `id`, not issue ID. Types: `relates`, `duplicates`, `duplicated`, `blocks`, `blocked`, `precedes`, `follows`, `copied_to`, `copied_from`.

### Create or update an issue

```bash
rdm issue create \
  --project 1 \
  --subject "Implement user authentication" \
  --description "Add OAuth2 support" \
  --tracker 1 \
  --priority 2 \
  --assigned-to 5
```

```bash
rdm issue update --id 123 --status 3 --done-ratio 50 --notes "Halfway done"

# Change tracker (must be enabled for the issue's project)
rdm issue update --id 123 --tracker 2
```

### Log or update time

```bash
rdm time create --issue 123 --hours 2.5 --activity Development --comment "Implemented feature X"
rdm time create --project 1 --hours 1.0 --activity Meeting --comment "Sprint planning"
```

If the activity is uncertain:

```bash
rdm time activities list
```

```bash
rdm time update --id 456 --hours 3.0 --comment "Adjusted after review"
rdm time delete --id 456
```

### Summaries and grouping

```bash
rdm time list --user me --from 2024-01-01 --to 2024-01-31
rdm time list --user me --from 2024-01-01 --to 2024-01-31 --group-by project
```

Supported `--group-by` values:

- `user`
- `project`
- `activity`
- `issue`
- `spent_on`
- `cf_<id>`

## Custom Fields

Use repeatable `--cf <id>=<value>` arguments for:

- issue filtering
- time-entry filtering
- issue create
- issue update
- time grouping by `cf_<id>`

Examples:

```bash
rdm issue create \
  --project 1 \
  --subject "High priority task" \
  --cf 5=urgent \
  --cf 6=backend
```

```bash
rdm issue update --id 123 --cf 5=normal --cf 7="Q2 2024"
```

```bash
rdm time list --from 2024-01-01 --to 2024-01-31 --cf 9=client-a
rdm time list --from 2024-01-01 --to 2024-01-31 --group-by cf_9
```

Rules:

- treat custom-field IDs as instance-specific
- do not invent IDs or allowed values
- if IDs are unknown, first inspect existing output or user-provided examples

## Agent Patterns

Prefer:

- `rdm ping` when auth or target instance is uncertain
- `rdm project list` or `rdm user list` to resolve IDs before writes
- `rdm issue list --format json` before `rdm issue get` when selecting one item from many
- `rdm time activities list` before `rdm time create` if activity naming is uncertain
- `rdm me` or `rdm user me` before using `me` in filters or assignments

Use `--dry-run` only for previewing write requests. In the current CLI, dry-run prints the request and exits with a validation error instead of success, so do not treat it as a successful no-op in scripts or agent loops.

## Error Handling

Stable exit codes:

- `0` success
- `2` validation or argument error
- `3` authentication or configuration error
- `4` resource not found
- `5` API, server, or network error

## Gotchas

- updating an issue before resolving the correct issue ID
- guessing project IDs, user IDs, status IDs, tracker IDs, priority IDs, activity IDs, or custom-field IDs
- using markdown output when exact downstream parsing is required
- assuming `--search` combines with all issue-list filters
- assuming `--dry-run` exits successfully
- running a project-scoped saved query without `--project` (returns 404)

//! MCP (Model Context Protocol) server over stdio.
//!
//! Exposes every `rdm` leaf subcommand as an MCP tool. Tool schemas are
//! generated from the clap command tree, and tool calls are translated back
//! into CLI arguments and dispatched through the same code path as the CLI,
//! so CLI and MCP never drift apart.

use std::any::TypeId;

use clap::{ArgAction, CommandFactory, Parser};
use serde_json::{json, Map, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::cli::Cli;
use crate::config::{load_config, ConfigPaths};
use crate::output::{Format, OutputFormat};

/// MCP protocol versions supported by this server, newest first.
/// The server only uses features (tools, ping) common to all of them.
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

/// Echo the client's version if supported, otherwise offer our latest.
fn negotiate_version(requested: Option<&str>) -> &'static str {
    requested
        .and_then(|r| SUPPORTED_PROTOCOL_VERSIONS.iter().find(|v| **v == r))
        .copied()
        .unwrap_or(SUPPORTED_PROTOCOL_VERSIONS[0])
}

/// Top-level commands not exposed as tools (local config/credential management).
const EXCLUDED_COMMANDS: &[&str] = &["profile", "config", "mcp", "help"];

/// Global CLI args that are fixed by the server process, not per tool call.
const GLOBAL_ARGS: &[&str] = &[
    "format", "url", "api_key", "debug", "dry_run", "help", "version",
];

/// A tool derived from a clap leaf subcommand.
struct Tool {
    name: String,
    /// Subcommand path, e.g. `["issue", "attachment", "list"]`.
    path: Vec<String>,
    description: String,
    input_schema: Value,
    read_only: bool,
}

/// Run the MCP server on stdin/stdout until stdin closes.
pub async fn serve(url: Option<String>, api_key: Option<String>) -> std::io::Result<()> {
    let tools = collect_tools();
    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = stdin.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(line) {
            Ok(msg) => handle_message(&msg, &tools, url.as_deref(), api_key.as_deref()).await,
            Err(e) => Some(error_response(
                Value::Null,
                -32700,
                &format!("Parse error: {e}"),
            )),
        };
        if let Some(resp) = response {
            let mut out = serde_json::to_string(&resp).unwrap_or_default();
            out.push('\n');
            stdout.write_all(out.as_bytes()).await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

async fn handle_message(
    msg: &Value,
    tools: &[Tool],
    url: Option<&str>,
    api_key: Option<&str>,
) -> Option<Value> {
    if !msg.is_object() || msg.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        return Some(error_response(
            id,
            -32600,
            "Invalid Request: expected a JSON-RPC 2.0 object",
        ));
    }
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    // Notifications (no id) never get a response.
    let id = msg.get("id")?.clone();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    let result = match method {
        "initialize" => {
            let negotiated =
                negotiate_version(params.get("protocolVersion").and_then(Value::as_str));
            Ok(json!({
                "protocolVersion": negotiated,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": {
                    "name": "rdm",
                    "title": "Redmine",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "instructions": "Redmine access via the rdm CLI. Tools mirror `rdm <command> <subcommand>`. \
                    Output is markdown by default; pass format=\"json\" for structured output. \
                    Use `ping` to verify connectivity, `time_activities_list` before logging time.",
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools.iter().map(tool_json).collect::<Vec<_>>() })),
        "tools/call" => call_tool(&params, tools, url, api_key).await,
        _ => Err((-32601, format!("Method not found: {method}"))),
    };

    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error_response(id, code, &message),
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn tool_json(tool: &Tool) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": tool.input_schema,
        "annotations": {
            "readOnlyHint": tool.read_only,
            "destructiveHint": tool.name.ends_with("_delete") || tool.name.ends_with("_remove"),
            "openWorldHint": true,
        },
    })
}

async fn call_tool(
    params: &Value,
    tools: &[Tool],
    url: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value, (i64, String)> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "Missing tool name".to_string()))?;
    let tool = tools
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| (-32602, format!("Unknown tool: {name}")))?;
    let empty = Map::new();
    let args = match params.get("arguments") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(map)) => map,
        Some(_) => return Err((-32602, "`arguments` must be an object".to_string())),
    };

    let (text, is_error) = match build_argv(tool, args) {
        Ok((argv, format)) => run_tool(argv, format, url, api_key).await,
        Err(msg) => (msg, true),
    };
    Ok(json!({ "content": [{ "type": "text", "text": text }], "isError": is_error }))
}

/// Execute a parsed tool call, returning (output text, is_error).
async fn run_tool(
    argv: Vec<String>,
    format: OutputFormat,
    url: Option<&str>,
    api_key: Option<&str>,
) -> (String, bool) {
    let cli = match Cli::try_parse_from(&argv) {
        Ok(cli) => cli,
        Err(e) => return (e.to_string(), true),
    };
    let result = async {
        let paths = ConfigPaths::new()?;
        let config = load_config(url, api_key, &paths)?;
        let client = crate::client::RedmineClient::new(&config, false)?;
        crate::execute_command(&cli.command, &client, &paths, format).await
    }
    .await;
    match result {
        Ok(out) => (out, false),
        Err(e) => (format.format_error(&e), true),
    }
}

/// Translate JSON tool arguments into a CLI argv.
fn build_argv(
    tool: &Tool,
    args: &Map<String, Value>,
) -> Result<(Vec<String>, OutputFormat), String> {
    let mut argv = vec!["rdm".to_string()];
    argv.extend(tool.path.iter().cloned());

    let mut format = OutputFormat::Markdown;
    let props = tool.input_schema["properties"]
        .as_object()
        .cloned()
        .unwrap_or_default();

    for (key, value) in args {
        if key == "format" {
            format = match value {
                Value::Null => OutputFormat::Markdown,
                Value::String(s) if s == "markdown" => OutputFormat::Markdown,
                Value::String(s) if s == "json" => OutputFormat::Json,
                other => {
                    return Err(format!(
                        "Invalid format: {other} (expected \"markdown\" or \"json\")"
                    ))
                }
            };
            continue;
        }
        if !props.contains_key(key) {
            return Err(format!("Unknown argument: {key}"));
        }
        // Property keys are long flags with '-' replaced by '_'.
        let flag = format!("--{}", key.replace('_', "-"));
        match value {
            Value::Null => {}
            Value::Bool(b) => {
                if *b {
                    argv.push(flag);
                }
            }
            Value::Array(items) => {
                // `--flag=value` so values starting with '-' aren't read as flags.
                for item in items {
                    argv.push(format!("{flag}={}", scalar_to_string(item)?));
                }
            }
            other => {
                // `--flag=value` keeps values starting with '-' from being read as flags.
                argv.push(format!("{flag}={}", scalar_to_string(other)?));
            }
        }
    }
    Ok((argv, format))
}

fn scalar_to_string(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        other => Err(format!("Unsupported argument value: {other}")),
    }
}

/// Walk the clap command tree and build one tool per leaf subcommand.
fn collect_tools() -> Vec<Tool> {
    let root = Cli::command();
    let mut tools = Vec::new();
    for sub in root.get_subcommands() {
        if EXCLUDED_COMMANDS.contains(&sub.get_name()) {
            continue;
        }
        walk(sub, vec![sub.get_name().to_string()], &mut tools);
    }
    tools
}

fn walk(cmd: &clap::Command, path: Vec<String>, tools: &mut Vec<Tool>) {
    let subs: Vec<_> = cmd
        .get_subcommands()
        .filter(|s| s.get_name() != "help")
        .collect();
    if !subs.is_empty() {
        for sub in subs {
            let mut p = path.clone();
            p.push(sub.get_name().to_string());
            walk(sub, p, tools);
        }
        return;
    }

    let mut properties = Map::new();
    let mut required = Vec::new();
    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if GLOBAL_ARGS.contains(&id) || arg.is_global_set() || arg.is_hide_set() {
            continue;
        }
        let Some(long) = arg.get_long() else {
            continue;
        };
        let key = long.replace('-', "_");
        let mut schema = arg_schema(arg);
        let mut desc = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
        if let Some(default) = arg.get_default_values().first() {
            desc.push_str(&format!(" (default: {})", default.to_string_lossy()));
        }
        schema["description"] = json!(desc.trim());
        if arg.is_required_set() {
            required.push(key.clone());
        }
        properties.insert(key, schema);
    }
    properties.insert(
        "format".to_string(),
        json!({
            "type": "string",
            "enum": ["markdown", "json"],
            "description": "Output format (default: markdown).",
        }),
    );

    let name = path.join("_");
    let leaf = path.last().map(String::as_str).unwrap_or("");
    // `download` writes to the local filesystem, so it is not read-only.
    let read_only = matches!(leaf, "list" | "get" | "me" | "ping");
    let about = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();
    tools.push(Tool {
        description: format!("{about} (`rdm {}`)", path.join(" ")),
        name,
        path,
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        }),
        read_only,
    });
}

fn arg_schema(arg: &clap::Arg) -> Value {
    let parser = arg.get_value_parser();
    let possible: Vec<String> = parser
        .possible_values()
        .map(|vals| {
            vals.filter(|v| !v.is_hide_set())
                .map(|v| v.get_name().to_string())
                .collect()
        })
        .unwrap_or_default();

    let type_id = parser.type_id();
    let scalar = if matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse) {
        return json!({ "type": "boolean" });
    } else if !possible.is_empty() {
        json!({ "type": "string", "enum": possible })
    } else if [
        TypeId::of::<u8>(),
        TypeId::of::<u16>(),
        TypeId::of::<u32>(),
        TypeId::of::<u64>(),
        TypeId::of::<i8>(),
        TypeId::of::<i16>(),
        TypeId::of::<i32>(),
        TypeId::of::<i64>(),
        TypeId::of::<usize>(),
    ]
    .iter()
    .any(|t| type_id == *t)
    {
        json!({ "type": "integer" })
    } else if type_id == TypeId::of::<f64>() || type_id == TypeId::of::<f32>() {
        json!({ "type": "number" })
    } else {
        json!({ "type": "string" })
    };

    if matches!(arg.get_action(), ArgAction::Append) {
        json!({ "type": "array", "items": scalar })
    } else {
        scalar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool<'a>(tools: &'a [Tool], name: &str) -> &'a Tool {
        tools
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("missing tool {name}"))
    }

    #[test]
    fn tools_exclude_profile_and_config() {
        let tools = collect_tools();
        assert!(tools
            .iter()
            .all(|t| !t.name.starts_with("profile") && t.name != "config"));
        assert!(tools.iter().any(|t| t.name == "issue_attachment_upload"));
        assert!(tools.iter().any(|t| t.name == "time_activities_list"));
    }

    #[test]
    fn schema_types_and_required() {
        let tools = collect_tools();
        let t = tool(&tools, "time_create");
        let props = &t.input_schema["properties"];
        assert_eq!(props["hours"]["type"], "number");
        assert_eq!(props["issue"]["type"], "integer");
        assert_eq!(props["activity"]["type"], "string");
        let req: Vec<_> = t.input_schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .collect();
        assert!(req.contains(&&json!("hours")));

        let t = tool(&tools, "issue_list");
        let props = &t.input_schema["properties"];
        assert_eq!(props["include_relations"]["type"], "boolean");
        assert_eq!(props["cf"]["type"], "array");
        assert_eq!(props["version"]["type"], "array");

        let t = tool(&tools, "issue_relation_add");
        assert!(t.input_schema["properties"]["type"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("blocks")));
    }

    #[test]
    fn argv_round_trips_through_clap() {
        let tools = collect_tools();
        let t = tool(&tools, "issue_list");
        let args = json!({
            "project": "demo",
            "assigned_to": "me",
            "cf": ["5=x", "6=y"],
            "include_relations": true,
            "limit": 10,
            "format": "json",
        });
        let (argv, format) = build_argv(t, args.as_object().unwrap()).unwrap();
        assert_eq!(format, OutputFormat::Json);
        Cli::try_parse_from(&argv).expect("argv parses");

        let t = tool(&tools, "issue_relation_add");
        let args = json!({ "id": 1, "to": 2, "type": "precedes", "delay": -3 });
        let (argv, _) = build_argv(t, args.as_object().unwrap()).unwrap();
        Cli::try_parse_from(&argv).expect("negative value parses");
    }

    async fn handle(msg: Value) -> Option<Value> {
        let tools = collect_tools();
        handle_message(&msg, &tools, None, None).await
    }

    #[tokio::test]
    async fn jsonrpc_initialize_and_list() {
        let resp = handle(json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-03-26" }
        }))
        .await
        .unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], "2025-03-26");
        assert!(resp["result"]["capabilities"]["tools"].is_object());

        let resp = handle(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))
            .await
            .unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "issue_list"));
    }

    #[tokio::test]
    async fn jsonrpc_notification_gets_no_response() {
        let msg = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(handle(msg).await.is_none());
    }

    #[tokio::test]
    async fn jsonrpc_invalid_envelope_rejected() {
        for msg in [
            json!({ "id": 1, "method": "ping" }),
            json!({ "jsonrpc": "1.0", "id": 1, "method": "ping" }),
            json!([1, 2]),
        ] {
            let resp = handle(msg).await.unwrap();
            assert_eq!(resp["error"]["code"], -32600);
        }
    }

    #[tokio::test]
    async fn jsonrpc_unknown_method_and_tool() {
        let resp = handle(json!({ "jsonrpc": "2.0", "id": 1, "method": "nope" }))
            .await
            .unwrap();
        assert_eq!(resp["error"]["code"], -32601);

        let resp = handle(json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "nope" }
        }))
        .await
        .unwrap();
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn jsonrpc_non_object_arguments_rejected() {
        let resp = handle(json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "ping", "arguments": 42 }
        }))
        .await
        .unwrap();
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[tokio::test]
    async fn jsonrpc_tool_validation_error_is_tool_error() {
        let resp = handle(json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "issue_get", "arguments": {} }
        }))
        .await
        .unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn repeated_values_starting_with_dash_parse() {
        let tools = collect_tools();
        let t = tool(&tools, "issue_list");
        let args = json!({ "cf": ["5=-1", "-6"] });
        let (argv, _) = build_argv(t, args.as_object().unwrap()).unwrap();
        assert!(argv.contains(&"--cf=-6".to_string()));
        Cli::try_parse_from(&argv).expect("dash-prefixed repeated value parses");
    }

    #[test]
    fn protocol_version_negotiation() {
        assert_eq!(negotiate_version(Some("2024-11-05")), "2024-11-05");
        assert_eq!(
            negotiate_version(Some("2099-01-01")),
            SUPPORTED_PROTOCOL_VERSIONS[0]
        );
        assert_eq!(negotiate_version(None), SUPPORTED_PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn invalid_format_rejected() {
        let tools = collect_tools();
        let t = tool(&tools, "ping");
        for bad in [json!(1), json!(true), json!(["json"]), json!("xml")] {
            let args = json!({ "format": bad });
            assert!(build_argv(t, args.as_object().unwrap()).is_err());
        }
        let args = json!({ "format": null });
        assert!(build_argv(t, args.as_object().unwrap()).is_ok());
    }

    #[test]
    fn download_is_not_read_only() {
        let tools = collect_tools();
        assert!(!tool(&tools, "issue_attachment_download").read_only);
        assert!(tool(&tools, "issue_list").read_only);
    }

    #[test]
    fn unknown_argument_rejected() {
        let tools = collect_tools();
        let t = tool(&tools, "ping");
        assert!(build_argv(t, json!({ "bogus": 1 }).as_object().unwrap()).is_err());
    }
}

//! CLI command definitions.

pub mod issue;
pub mod ping;
pub mod profile;
pub mod project;
pub mod query;
pub mod time;
pub mod user;
pub mod version;

use crate::error::{AppError, Result};
use crate::output::OutputFormat;
use clap::{Parser, Subcommand};

/// Parse custom field arguments in format "id=value".
pub fn parse_custom_fields(args: &[String]) -> Result<Vec<(u32, String)>> {
    let mut result = Vec::new();
    for arg in args {
        let parts: Vec<&str> = arg.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(AppError::validation_with_hint(
                format!("Invalid custom field format: '{}'", arg),
                "Use format: --cf 5=value",
            ));
        }
        let id: u32 = parts[0].parse().map_err(|_| {
            AppError::validation_with_hint(
                format!("Invalid custom field ID: '{}'", parts[0]),
                "Custom field ID must be a number, e.g., --cf 5=value",
            )
        })?;
        result.push((id, parts[1].to_string()));
    }
    Ok(result)
}

/// Agent-first Redmine CLI with markdown-first output.
#[derive(Debug, Parser)]
#[command(name = "rdm", version, about, long_about = None)]
pub struct Cli {
    /// Output format (markdown or json).
    #[arg(
        long,
        short = 'f',
        value_enum,
        default_value = "markdown",
        global = true
    )]
    pub format: OutputFormat,

    /// Redmine server URL (overrides env/config).
    #[arg(long, env = "REDMINE_URL", global = true)]
    pub url: Option<String>,

    /// Redmine API key (overrides env/config).
    #[arg(long, env = "REDMINE_API_KEY", global = true)]
    pub api_key: Option<String>,

    /// Enable debug output to stderr.
    #[arg(long, global = true)]
    pub debug: bool,

    /// Print request without executing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check connection and authentication.
    Ping,

    /// Show current user information.
    Me,

    /// Manage configuration profiles.
    #[command(subcommand)]
    Profile(profile::ProfileCommand),

    /// Show current configuration.
    Config(profile::ConfigShow),

    /// Project commands.
    #[command(subcommand)]
    Project(project::ProjectCommand),

    /// Issue commands.
    #[command(subcommand)]
    Issue(issue::IssueCommand),

    /// Saved query commands.
    #[command(subcommand)]
    Query(query::QueryCommand),

    /// Time entry commands.
    #[command(subcommand)]
    Time(time::TimeCommand),

    /// User commands.
    #[command(subcommand)]
    User(user::UserCommand),

    /// Version (target version / milestone) commands.
    #[command(subcommand)]
    Version(version::VersionCommand),

    /// Run as an MCP (Model Context Protocol) server over stdio.
    ///
    /// Exposes rdm commands as tools for MCP clients (Claude Desktop/Cowork,
    /// Claude Code, ...). Credentials come from --url/--api-key, env, or the
    /// active profile, same as the CLI.
    Mcp,
}

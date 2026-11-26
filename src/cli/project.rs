//! Project commands.

use clap::{Args, Subcommand};

use super::parse_custom_fields;
use crate::client::RedmineClient;
use crate::error::Result;
use crate::models::{Project, ProjectList};

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// List projects.
    List(ProjectListArgs),
    /// Get project details.
    Get(ProjectGetArgs),
}

#[derive(Debug, Args)]
pub struct ProjectListArgs {
    /// Filter by project custom field value (format: id=value, repeatable).
    /// The field must be enabled as a filter in Redmine (Redmine 4.1+).
    #[arg(long = "cf", value_name = "ID=VALUE")]
    pub custom_fields: Vec<String>,
    /// Maximum number of results.
    #[arg(long, default_value = "25")]
    pub limit: u32,
    /// Offset for pagination.
    #[arg(long, default_value = "0")]
    pub offset: u32,
}

#[derive(Debug, Args)]
pub struct ProjectGetArgs {
    /// Project ID.
    #[arg(long, conflicts_with = "identifier")]
    pub id: Option<u32>,
    /// Project identifier (slug).
    #[arg(long, conflicts_with = "id")]
    pub identifier: Option<String>,
}

/// Execute project list command.
pub async fn list(client: &RedmineClient, args: &ProjectListArgs) -> Result<ProjectList> {
    let custom_fields = parse_custom_fields(&args.custom_fields)?;
    client
        .list_projects(args.limit, args.offset, &custom_fields)
        .await
}

/// Execute project get command.
pub async fn get(client: &RedmineClient, args: &ProjectGetArgs) -> Result<Project> {
    let id_or_identifier = if let Some(id) = args.id {
        id.to_string()
    } else if let Some(identifier) = &args.identifier {
        identifier.clone()
    } else {
        return Err(crate::error::AppError::validation_with_hint(
            "Either --id or --identifier is required",
            "Use `rdm project get --id 1` or `rdm project get --identifier my-project`",
        ));
    };

    client.get_project(&id_or_identifier).await
}

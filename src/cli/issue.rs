//! Issue commands.

use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;

use super::parse_custom_fields;
use super::version::resolve_version_ids;
use crate::client::{endpoints::IssueFilters, RedmineClient};
use crate::error::{AppError, Result};
use crate::models::{
    attachment::{guess_content_type, AttachmentRef},
    AttachmentDownloaded, AttachmentList, AttachmentUploaded, ClearableId, CustomFieldValue, Issue,
    IssueList, NewIssue, NewRelation, RelationCreated, RelationDeleted, RelationList, UpdateIssue,
};
use crate::output::{markdown::markdown_kv_table, MarkdownOutput, Meta};

#[derive(Debug, Subcommand)]
pub enum IssueCommand {
    /// List issues.
    List(IssueListArgs),
    /// Get issue details.
    Get(IssueGetArgs),
    /// Create a new issue.
    Create(IssueCreateArgs),
    /// Update an issue.
    Update(IssueUpdateArgs),
    /// Attachment commands.
    #[command(subcommand)]
    Attachment(AttachmentCommand),
    /// Relation commands (blocks, precedes, relates, ...).
    #[command(subcommand)]
    Relation(RelationCommand),
}

#[derive(Debug, Subcommand)]
pub enum RelationCommand {
    /// List relations of an issue.
    List(RelationListArgs),
    /// Add a relation from an issue to another issue.
    Add(RelationAddArgs),
    /// Remove a relation by its relation ID.
    Remove(RelationRemoveArgs),
}

#[derive(Debug, Args)]
pub struct RelationListArgs {
    /// Issue ID.
    #[arg(long)]
    pub id: u32,
}

/// Redmine relation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RelationType {
    Relates,
    Duplicates,
    Duplicated,
    Blocks,
    Blocked,
    Precedes,
    Follows,
    #[value(name = "copied_to", alias = "copied-to")]
    CopiedTo,
    #[value(name = "copied_from", alias = "copied-from")]
    CopiedFrom,
}

impl RelationType {
    /// Redmine API value.
    pub fn as_api(self) -> &'static str {
        match self {
            Self::Relates => "relates",
            Self::Duplicates => "duplicates",
            Self::Duplicated => "duplicated",
            Self::Blocks => "blocks",
            Self::Blocked => "blocked",
            Self::Precedes => "precedes",
            Self::Follows => "follows",
            Self::CopiedTo => "copied_to",
            Self::CopiedFrom => "copied_from",
        }
    }
}

#[derive(Debug, Args)]
pub struct RelationAddArgs {
    /// Source issue ID.
    #[arg(long)]
    pub id: u32,
    /// Target issue ID.
    #[arg(long)]
    pub to: u32,
    /// Relation type (from the source issue's perspective).
    #[arg(long = "type", value_enum)]
    pub relation_type: RelationType,
    /// Delay in days (only for precedes/follows).
    #[arg(long, allow_hyphen_values = true)]
    pub delay: Option<i32>,
}

#[derive(Debug, Args)]
pub struct RelationRemoveArgs {
    /// Relation ID (see `rdm issue relation list --id <ISSUE>`).
    #[arg(long)]
    pub relation_id: u32,
}

#[derive(Debug, Subcommand)]
pub enum AttachmentCommand {
    /// List attachments on an issue.
    List(AttachmentListArgs),
    /// Download an attachment by ID.
    Download(AttachmentDownloadArgs),
    /// Upload a file and attach it to an issue.
    Upload(AttachmentUploadArgs),
}

#[derive(Debug, Args)]
pub struct AttachmentListArgs {
    /// Issue ID.
    #[arg(long)]
    pub issue_id: u32,
}

#[derive(Debug, Args)]
pub struct AttachmentDownloadArgs {
    /// Attachment ID.
    #[arg(long)]
    pub id: u32,
    /// Output path (default: current directory, filename from attachment).
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct AttachmentUploadArgs {
    /// Issue ID to attach the file to.
    #[arg(long)]
    pub issue_id: u32,
    /// Path to the file to upload.
    #[arg(long)]
    pub file: PathBuf,
    /// Override filename (default: file's basename).
    #[arg(long)]
    pub filename: Option<String>,
    /// Optional description for the attachment.
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct IssueListArgs {
    /// Filter by project (ID or identifier).
    #[arg(long)]
    pub project: Option<String>,
    /// Filter by status (ID, "open", "closed", or "*").
    #[arg(long)]
    pub status: Option<String>,
    /// Filter by assignee (ID or "me").
    #[arg(long)]
    pub assigned_to: Option<String>,
    /// Filter by author (ID or "me").
    #[arg(long)]
    pub author: Option<String>,
    /// Filter by tracker ID.
    #[arg(long)]
    pub tracker: Option<String>,
    /// Filter by target version (ID or name; repeatable or comma-separated).
    /// Names are resolved within `--project`.
    #[arg(long = "version", value_name = "ID|NAME", value_delimiter = ',')]
    pub versions: Vec<String>,
    /// Filter by exact subject match.
    #[arg(long)]
    pub subject: Option<String>,
    /// Search issues by text (searches subject and description).
    #[arg(long)]
    pub search: Option<String>,
    /// Run a saved query by ID (see `rdm query list`). For a project-scoped
    /// query, also pass `--project` with its project.
    #[arg(long)]
    pub query_id: Option<u32>,
    /// Filter by custom field value (format: id=value, repeatable).
    #[arg(long = "cf", value_name = "ID=VALUE")]
    pub custom_fields: Vec<String>,
    /// Include each issue's relations in the output (`include=relations`).
    #[arg(long)]
    pub include_relations: bool,
    /// Maximum number of results.
    #[arg(long, default_value = "25")]
    pub limit: u32,
    /// Offset for pagination.
    #[arg(long, default_value = "0")]
    pub offset: u32,
}

#[derive(Debug, Args)]
pub struct IssueGetArgs {
    /// Issue ID.
    #[arg(long)]
    pub id: u32,
}

#[derive(Debug, Args)]
pub struct IssueCreateArgs {
    /// Project ID.
    #[arg(long)]
    pub project: u32,
    /// Issue subject.
    #[arg(long)]
    pub subject: String,
    /// Issue description.
    #[arg(long)]
    pub description: Option<String>,
    /// Tracker ID.
    #[arg(long)]
    pub tracker: Option<u32>,
    /// Status ID.
    #[arg(long)]
    pub status: Option<u32>,
    /// Priority ID.
    #[arg(long)]
    pub priority: Option<u32>,
    /// Assignee ID.
    #[arg(long)]
    pub assigned_to: Option<u32>,
    /// Start date (YYYY-MM-DD).
    #[arg(long)]
    pub start_date: Option<String>,
    /// Due date (YYYY-MM-DD).
    #[arg(long)]
    pub due_date: Option<String>,
    /// Estimated hours.
    #[arg(long)]
    pub estimated_hours: Option<f64>,
    /// Parent issue ID.
    #[arg(long)]
    pub parent: Option<u32>,
    /// Set custom field value (format: id=value, repeatable).
    #[arg(long = "cf", value_name = "ID=VALUE")]
    pub custom_fields: Vec<String>,
}

#[derive(Debug, Args)]
pub struct IssueUpdateArgs {
    /// Issue ID.
    #[arg(long)]
    pub id: u32,
    /// New subject.
    #[arg(long)]
    pub subject: Option<String>,
    /// New description.
    #[arg(long)]
    pub description: Option<String>,
    /// New tracker ID (must be enabled for the issue's project).
    #[arg(long)]
    pub tracker: Option<u32>,
    /// New status ID.
    #[arg(long)]
    pub status: Option<u32>,
    /// New priority ID.
    #[arg(long)]
    pub priority: Option<u32>,
    /// New assignee ID, or `none` to unassign.
    #[arg(long, value_name = "ID|none", value_parser = ClearableId::parse)]
    pub assigned_to: Option<ClearableId>,
    /// New target version (ID or name within the issue's project).
    #[arg(long, value_name = "ID|NAME")]
    pub version: Option<String>,
    /// New parent issue ID, or `none` to remove the parent.
    #[arg(long, value_name = "ID|none", value_parser = ClearableId::parse)]
    pub parent: Option<ClearableId>,
    /// Done percentage (0-100).
    #[arg(long)]
    pub done_ratio: Option<u32>,
    /// Add a note/comment.
    #[arg(long)]
    pub notes: Option<String>,
    /// Set custom field value (format: id=value, repeatable).
    #[arg(long = "cf", value_name = "ID=VALUE")]
    pub custom_fields: Vec<String>,
}

/// Result of issue creation.
#[derive(Debug, Clone, Serialize)]
pub struct IssueCreated {
    pub issue: Issue,
}

impl MarkdownOutput for IssueCreated {
    fn to_markdown(&self, _meta: &Meta) -> String {
        let i = &self.issue;
        let mut output = String::new();
        output.push_str("## Issue Created\n\n");

        let pairs = [
            ("ID", i.id.to_string()),
            ("Subject", i.subject.clone()),
            ("Project", i.project.name.clone()),
            ("Status", i.status.name.clone()),
            ("Priority", i.priority.name.clone()),
        ];

        let pairs_ref: Vec<(&str, String)> = pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        output.push_str(&markdown_kv_table(&pairs_ref));

        output.push_str(&format!(
            "\n*Use `rdm issue get --id {}` to view full details*\n",
            i.id
        ));
        output
    }
}

/// Result of issue update.
#[derive(Debug, Clone, Serialize)]
pub struct IssueUpdated {
    pub id: u32,
}

impl MarkdownOutput for IssueUpdated {
    fn to_markdown(&self, _meta: &Meta) -> String {
        format!("## Issue Updated\n\nIssue #{} has been updated.\n\n*Use `rdm issue get --id {}` to view changes*\n", self.id, self.id)
    }
}

/// Parse custom field arguments into CustomFieldValue vec, or None if empty.
fn parse_custom_field_values(args: &[String]) -> Result<Option<Vec<CustomFieldValue>>> {
    if args.is_empty() {
        Ok(None)
    } else {
        let parsed = parse_custom_fields(args)?;
        Ok(Some(CustomFieldValue::from_tuples(parsed)))
    }
}

/// Execute issue list command.
pub async fn list(client: &RedmineClient, args: &IssueListArgs) -> Result<IssueList> {
    // Parse custom field filters
    let custom_fields = parse_custom_fields(&args.custom_fields)?;

    let filters = IssueFilters {
        project: args.project.clone(),
        status: args.status.clone(),
        assigned_to: args.assigned_to.clone(),
        author: args.author.clone(),
        tracker: args.tracker.clone(),
        fixed_versions: resolve_version_ids(client, args.project.as_deref(), &args.versions)
            .await?,
        subject: args.subject.clone(),
        query_id: args.query_id,
        custom_fields,
        include_relations: args.include_relations,
        limit: args.limit,
        offset: args.offset,
    };

    // If search is specified, use search endpoint instead
    if let Some(query) = &args.search {
        return client
            .search_issues(query, args.project.as_deref(), args.limit, args.offset)
            .await;
    }

    client.list_issues(filters).await
}

/// Execute issue get command.
pub async fn get(client: &RedmineClient, args: &IssueGetArgs) -> Result<Issue> {
    client.get_issue(args.id).await
}

/// Execute issue create command.
pub async fn create(client: &RedmineClient, args: &IssueCreateArgs) -> Result<IssueCreated> {
    let custom_fields = parse_custom_field_values(&args.custom_fields)?;

    let issue = NewIssue {
        project_id: args.project,
        subject: args.subject.clone(),
        description: args.description.clone(),
        tracker_id: args.tracker,
        status_id: args.status,
        priority_id: args.priority,
        assigned_to_id: args.assigned_to,
        start_date: args.start_date.clone(),
        due_date: args.due_date.clone(),
        estimated_hours: args.estimated_hours,
        parent_issue_id: args.parent,
        custom_fields,
    };

    let created = client.create_issue(issue).await?;
    Ok(IssueCreated { issue: created })
}

/// Execute issue update command.
pub async fn update(client: &RedmineClient, args: &IssueUpdateArgs) -> Result<IssueUpdated> {
    let custom_fields = parse_custom_field_values(&args.custom_fields)?;

    let fixed_version_id = match &args.version {
        Some(v) => match v.trim().parse::<u32>() {
            Ok(id) => Some(id),
            // Names resolve within the issue's own project.
            Err(_) => {
                let project = client.get_issue(args.id).await?.project.id.to_string();
                resolve_version_ids(client, Some(&project), std::slice::from_ref(v))
                    .await?
                    .first()
                    .copied()
            }
        },
        None => None,
    };

    let update = UpdateIssue {
        subject: args.subject.clone(),
        description: args.description.clone(),
        tracker_id: args.tracker,
        status_id: args.status,
        priority_id: args.priority,
        assigned_to_id: args.assigned_to,
        fixed_version_id,
        parent_issue_id: args.parent,
        done_ratio: args.done_ratio,
        notes: args.notes.clone(),
        custom_fields,
        ..Default::default()
    };

    client.update_issue(args.id, update).await?;
    Ok(IssueUpdated { id: args.id })
}

/// List relations of an issue.
pub async fn relation_list(
    client: &RedmineClient,
    args: &RelationListArgs,
) -> Result<RelationList> {
    let relations = client.list_relations(args.id).await?;
    Ok(RelationList {
        issue_id: args.id,
        relations,
    })
}

/// Add a relation between two issues.
pub async fn relation_add(
    client: &RedmineClient,
    args: &RelationAddArgs,
) -> Result<RelationCreated> {
    let relation = NewRelation {
        issue_to_id: args.to,
        relation_type: args.relation_type.as_api().to_string(),
        delay: args.delay,
    };
    let relation = client.create_relation(args.id, relation).await?;
    Ok(RelationCreated { relation })
}

/// Remove a relation.
pub async fn relation_remove(
    client: &RedmineClient,
    args: &RelationRemoveArgs,
) -> Result<RelationDeleted> {
    client.delete_relation(args.relation_id).await?;
    Ok(RelationDeleted {
        relation_id: args.relation_id,
        deleted: true,
    })
}

/// List attachments on an issue.
pub async fn attachment_list(
    client: &RedmineClient,
    args: &AttachmentListArgs,
) -> Result<AttachmentList> {
    let issue = client.get_issue(args.issue_id).await?;
    Ok(AttachmentList {
        issue_id: args.issue_id,
        attachments: issue.attachments.unwrap_or_default(),
    })
}

/// Download an attachment.
pub async fn attachment_download(
    client: &RedmineClient,
    args: &AttachmentDownloadArgs,
) -> Result<AttachmentDownloaded> {
    let attachment = client.get_attachment(args.id).await?;
    let bytes = client.download_attachment(&attachment.content_url).await?;

    let output_path = match &args.output {
        Some(p) if p.is_dir() => p.join(&attachment.filename),
        Some(p) => p.clone(),
        None => PathBuf::from(&attachment.filename),
    };

    tokio::fs::write(&output_path, &bytes).await.map_err(|e| {
        AppError::api(
            format!("Failed to write {}: {}", output_path.display(), e),
            None,
        )
    })?;

    Ok(AttachmentDownloaded {
        id: attachment.id,
        filename: attachment.filename,
        saved_to: output_path,
        bytes: bytes.len() as u64,
    })
}

/// Upload a file and attach it to an issue.
pub async fn attachment_upload(
    client: &RedmineClient,
    args: &AttachmentUploadArgs,
) -> Result<AttachmentUploaded> {
    if !args.file.exists() {
        return Err(AppError::validation(format!(
            "File not found: {}",
            args.file.display()
        )));
    }

    let filename = args
        .filename
        .clone()
        .or_else(|| {
            args.file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "attachment".to_string());

    let content_type = guess_content_type(&args.file).to_string();
    let bytes = tokio::fs::read(&args.file).await.map_err(|e| {
        AppError::api(
            format!("Failed to read {}: {}", args.file.display(), e),
            None,
        )
    })?;

    let token = client.upload_file(bytes, &filename).await?;

    let upload_ref = AttachmentRef {
        token,
        filename: filename.clone(),
        content_type,
        description: args.description.clone(),
    };

    let update = UpdateIssue {
        uploads: Some(vec![upload_ref]),
        ..Default::default()
    };

    client.update_issue(args.issue_id, update).await?;

    Ok(AttachmentUploaded {
        filename,
        issue_id: args.issue_id,
    })
}

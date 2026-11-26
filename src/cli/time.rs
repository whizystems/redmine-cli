//! Time entry commands.

use chrono::Local;
use clap::{Args, Subcommand};
use serde::Serialize;

use super::parse_custom_fields;
use crate::cache::{resolve_activity, ActivityCache};
use crate::client::{endpoints::TimeEntryFilters, RedmineClient};
use crate::config::ConfigPaths;
use crate::error::{AppError, Result};
use crate::models::{
    ActivityList, GroupByField, GroupedTimeEntries, NewTimeEntry, TimeEntry, TimeEntryCreated,
    TimeEntryDeleted, TimeEntryList, TimeEntryUpdated, UpdateTimeEntry,
};
use crate::output::{MarkdownOutput, Meta};

#[derive(Debug, Subcommand)]
pub enum TimeCommand {
    /// List or manage activities.
    #[command(subcommand)]
    Activities(ActivitiesCommand),
    /// Create a time entry.
    Create(TimeCreateArgs),
    /// List time entries.
    List(TimeListArgs),
    /// Get time entry details.
    Get(TimeGetArgs),
    /// Update a time entry.
    Update(TimeUpdateArgs),
    /// Delete a time entry.
    Delete(TimeDeleteArgs),
}

#[derive(Debug, Subcommand)]
pub enum ActivitiesCommand {
    /// List available activities.
    List(ActivitiesListArgs),
}

#[derive(Debug, Args)]
pub struct ActivitiesListArgs {
    /// Force refresh from server (ignore cache).
    #[arg(long)]
    pub refresh: bool,
}

#[derive(Debug, Args)]
pub struct TimeCreateArgs {
    /// Issue ID.
    #[arg(long, conflicts_with = "project")]
    pub issue: Option<u32>,
    /// Project ID (if not logging against an issue).
    #[arg(long, conflicts_with = "issue")]
    pub project: Option<u32>,
    /// Hours spent.
    #[arg(long)]
    pub hours: f64,
    /// Activity name or ID.
    #[arg(long)]
    pub activity: String,
    /// Date spent (YYYY-MM-DD, defaults to today).
    #[arg(long)]
    pub spent_on: Option<String>,
    /// Comment.
    #[arg(long)]
    pub comment: Option<String>,
    /// User ID (for admins logging time for others).
    #[arg(long)]
    pub user: Option<u32>,
}

#[derive(Debug, Args)]
pub struct TimeListArgs {
    /// Filter by project (ID or identifier).
    #[arg(long)]
    pub project: Option<String>,
    /// Filter by issue ID.
    #[arg(long)]
    pub issue: Option<u32>,
    /// Filter by user ID or "me".
    #[arg(long)]
    pub user: Option<String>,
    /// Filter from date (YYYY-MM-DD).
    #[arg(long)]
    pub from: Option<String>,
    /// Filter to date (YYYY-MM-DD).
    #[arg(long)]
    pub to: Option<String>,
    /// Run a saved query by ID (see `rdm query list --type time`). For a
    /// project-scoped query, also pass `--project` with its project.
    #[arg(long)]
    pub query_id: Option<u32>,
    /// Filter by custom field value (format: id=value, repeatable).
    #[arg(long = "cf", value_name = "ID=VALUE")]
    pub custom_fields: Vec<String>,
    /// Group results by field (user, project, activity, issue, spent_on, or cf_<id>).
    #[arg(long)]
    pub group_by: Option<String>,
    /// Maximum number of results.
    #[arg(long, default_value = "25")]
    pub limit: u32,
    /// Offset for pagination.
    #[arg(long, default_value = "0")]
    pub offset: u32,
}

#[derive(Debug, Args)]
pub struct TimeGetArgs {
    /// Time entry ID.
    #[arg(long)]
    pub id: u32,
}

#[derive(Debug, Args)]
pub struct TimeUpdateArgs {
    /// Time entry ID.
    #[arg(long)]
    pub id: u32,
    /// New hours.
    #[arg(long)]
    pub hours: Option<f64>,
    /// New activity (name or ID).
    #[arg(long)]
    pub activity: Option<String>,
    /// New date (YYYY-MM-DD).
    #[arg(long)]
    pub spent_on: Option<String>,
    /// New comment.
    #[arg(long)]
    pub comment: Option<String>,
}

#[derive(Debug, Args)]
pub struct TimeDeleteArgs {
    /// Time entry ID.
    #[arg(long)]
    pub id: u32,
}

/// Get the cache file path.
fn cache_path(paths: &ConfigPaths) -> std::path::PathBuf {
    paths.cache_dir.join("activities.json")
}

/// Load or fetch activities, using cache when valid.
async fn get_activities(
    client: &RedmineClient,
    paths: &ConfigPaths,
    force_refresh: bool,
) -> Result<(ActivityList, bool)> {
    let cache_file = cache_path(paths);

    // Try loading from cache
    if !force_refresh {
        if let Ok(Some(cache)) = ActivityCache::load(&cache_file) {
            if cache.is_valid() {
                return Ok((
                    ActivityList {
                        time_entry_activities: cache.activities,
                    },
                    true,
                ));
            }
        }
    }

    // Fetch from server
    let activities = client.list_activities().await?;

    // Update cache. Never cache an empty list: `--dry-run` returns one without
    // contacting Redmine, and caching it would break later real writes.
    if !activities.time_entry_activities.is_empty() {
        let cache = ActivityCache::new(activities.time_entry_activities.clone());
        let _ = cache.save(&cache_file);
    }

    Ok((activities, false))
}

/// Execute activities list command.
pub async fn list_activities(
    client: &RedmineClient,
    paths: &ConfigPaths,
    args: &ActivitiesListArgs,
) -> Result<ActivityList> {
    let (activities, _from_cache) = get_activities(client, paths, args.refresh).await?;
    Ok(activities)
}

/// Execute time create command.
pub async fn create(
    client: &RedmineClient,
    paths: &ConfigPaths,
    args: &TimeCreateArgs,
) -> Result<TimeEntryCreated> {
    // Validate hours
    if args.hours <= 0.0 {
        return Err(AppError::validation_with_hint(
            "Hours must be positive",
            "Use a positive number like `--hours 2.5`",
        ));
    }

    // Validate issue or project
    if args.issue.is_none() && args.project.is_none() {
        return Err(AppError::validation_with_hint(
            "Either --issue or --project is required",
            "Use `--issue 123` to log time against an issue or `--project 1` for project-level time",
        ));
    }

    // Resolve activity
    let (activities, _) = get_activities(client, paths, false).await?;
    let cache = ActivityCache::new(activities.time_entry_activities);
    let activity_id = resolve_activity(&cache, &args.activity)?;

    // Default to today
    let spent_on = args
        .spent_on
        .clone()
        .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string());

    let entry = NewTimeEntry {
        issue_id: args.issue,
        project_id: args.project,
        hours: args.hours,
        activity_id,
        spent_on: Some(spent_on),
        comments: args.comment.clone(),
        user_id: args.user,
    };

    let created = client.create_time_entry(entry).await?;
    Ok(TimeEntryCreated {
        time_entry: created,
    })
}

/// Execute time list command.
pub async fn list(client: &RedmineClient, args: &TimeListArgs) -> Result<TimeListResult> {
    // Parse custom field filters
    let custom_fields = parse_custom_fields(&args.custom_fields)?;

    let filters = TimeEntryFilters {
        project: args.project.clone(),
        issue: args.issue,
        user: args.user.clone(),
        from: args.from.clone(),
        to: args.to.clone(),
        query_id: args.query_id,
        custom_fields,
        limit: args.limit,
        offset: args.offset,
    };
    let entries = client.list_time_entries(filters).await?;

    // If grouping is requested, group the results
    if let Some(group_by_str) = &args.group_by {
        let group_by = GroupByField::parse(group_by_str).ok_or_else(|| {
            AppError::validation_with_hint(
                format!("Invalid group-by field: '{}'", group_by_str),
                "Valid values: user, project, activity, issue, spent_on, cf_<id>",
            )
        })?;

        let grouped = GroupedTimeEntries::from_entries(entries.time_entries, &group_by);
        return Ok(TimeListResult::Grouped(grouped));
    }

    Ok(TimeListResult::List(entries))
}

/// Result of time list command - either grouped or ungrouped.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TimeListResult {
    List(TimeEntryList),
    Grouped(GroupedTimeEntries),
}

impl TimeListResult {
    /// Get pagination metadata.
    pub fn meta(&self) -> Meta {
        match self {
            TimeListResult::List(list) => Meta::paginated(
                list.total_count.unwrap_or(0),
                list.limit.unwrap_or(25),
                list.offset.unwrap_or(0),
            ),
            TimeListResult::Grouped(grouped) => Meta::paginated(grouped.total_count, 0, 0),
        }
    }
}

impl MarkdownOutput for TimeListResult {
    fn to_markdown(&self, meta: &Meta) -> String {
        match self {
            TimeListResult::List(list) => list.to_markdown(meta),
            TimeListResult::Grouped(grouped) => grouped.to_markdown(meta),
        }
    }
}

/// Execute time get command.
pub async fn get(client: &RedmineClient, args: &TimeGetArgs) -> Result<TimeEntry> {
    client.get_time_entry(args.id).await
}

/// Execute time update command.
pub async fn update(
    client: &RedmineClient,
    paths: &ConfigPaths,
    args: &TimeUpdateArgs,
) -> Result<TimeEntryUpdated> {
    // Resolve activity if provided
    let activity_id = if let Some(activity) = &args.activity {
        let (activities, _) = get_activities(client, paths, false).await?;
        let cache = ActivityCache::new(activities.time_entry_activities);
        Some(resolve_activity(&cache, activity)?)
    } else {
        None
    };

    let update = UpdateTimeEntry {
        hours: args.hours,
        activity_id,
        spent_on: args.spent_on.clone(),
        comments: args.comment.clone(),
    };

    let updated = client.update_time_entry(args.id, update).await?;
    Ok(TimeEntryUpdated {
        time_entry: updated,
    })
}

/// Execute time delete command.
pub async fn delete(client: &RedmineClient, args: &TimeDeleteArgs) -> Result<TimeEntryDeleted> {
    client.delete_time_entry(args.id).await?;
    Ok(TimeEntryDeleted { id: args.id })
}

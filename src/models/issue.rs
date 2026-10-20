//! Issue model with related types.

use super::attachment::{format_bytes, Attachment, AttachmentRef};
use super::custom_field::{CustomField, CustomFieldValue};
use super::project::ProjectRef;
use super::relation::{IssueRef, Relation};
use super::user::User;
use super::version::VersionRef;
use crate::output::{
    markdown::{markdown_kv_table, markdown_table, pagination_hint},
    MarkdownOutput, Meta,
};
use serde::{Deserialize, Serialize};

/// Tracker (Bug, Feature, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracker {
    pub id: u32,
    pub name: String,
}

/// Issue status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub is_closed: Option<bool>,
}

/// Issue priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Priority {
    pub id: u32,
    pub name: String,
}

/// A single field change within a journal entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalDetail {
    pub property: String,
    pub name: String,
    #[serde(default)]
    pub old_value: Option<String>,
    #[serde(default)]
    pub new_value: Option<String>,
}

/// A journal entry (comment + field changes) on an issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journal {
    pub id: u32,
    pub user: User,
    pub created_on: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub details: Vec<JournalDetail>,
}

/// Issue from Redmine API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u32,
    pub subject: String,
    #[serde(default)]
    pub description: Option<String>,
    pub project: ProjectRef,
    #[serde(default)]
    pub tracker: Option<Tracker>,
    pub status: Status,
    pub priority: Priority,
    #[serde(default)]
    pub author: Option<User>,
    #[serde(default)]
    pub assigned_to: Option<User>,
    /// Target version.
    #[serde(default)]
    pub fixed_version: Option<VersionRef>,
    /// Parent issue (omitted when the issue has no parent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<IssueRef>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub done_ratio: Option<u32>,
    #[serde(default)]
    pub estimated_hours: Option<f64>,
    #[serde(default)]
    pub spent_hours: Option<f64>,
    #[serde(default)]
    pub created_on: Option<String>,
    #[serde(default)]
    pub updated_on: Option<String>,
    #[serde(default)]
    pub custom_fields: Option<Vec<CustomField>>,
    #[serde(default)]
    pub journals: Option<Vec<Journal>>,
    #[serde(default)]
    pub attachments: Option<Vec<Attachment>>,
    /// Relations (present when requested via `include=relations`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relations: Option<Vec<Relation>>,
}

/// List of issues from API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueList {
    pub issues: Vec<Issue>,
    #[serde(default)]
    pub total_count: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Wrapper for single issue response.
#[derive(Debug, Deserialize)]
pub struct IssueResponse {
    pub issue: Issue,
}

/// New issue creation request.
#[derive(Debug, Clone, Serialize)]
pub struct NewIssue {
    pub project_id: u32,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_issue_id: Option<u32>,
    /// Custom field values for the issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<CustomFieldValue>>,
}

/// Wrapper for issue creation request.
#[derive(Debug, Serialize)]
pub struct NewIssueRequest {
    pub issue: NewIssue,
}

/// Issue update request.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateIssue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Set or clear the assignee.
    pub assigned_to_id: Option<ClearableId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_version_id: Option<u32>,
    /// Set or clear the parent issue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_issue_id: Option<ClearableId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_ratio: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Custom field values to update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<CustomFieldValue>>,
    /// Attachments to add (upload tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploads: Option<Vec<AttachmentRef>>,
}

/// ID field change for an update: set to an ID, or clear it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearableId {
    /// Set the field to this ID.
    Set(u32),
    /// Clear the field. Redmine clears ID fields on a blank value.
    Clear,
}

impl ClearableId {
    /// Parse `<ID>`, `#<ID>`, or `none` (clear).
    pub fn parse(value: &str) -> std::result::Result<Self, String> {
        let v = value.trim();
        if v.is_empty() || v.eq_ignore_ascii_case("none") {
            return Ok(Self::Clear);
        }
        v.trim_start_matches('#')
            .parse::<u32>()
            .map(Self::Set)
            .map_err(|_| format!("invalid value '{}': expected an ID or 'none'", value))
    }
}

impl Serialize for ClearableId {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Set(id) => serializer.serialize_u32(*id),
            Self::Clear => serializer.serialize_str(""),
        }
    }
}

/// Wrapper for issue update request.
#[derive(Debug, Serialize)]
pub struct UpdateIssueRequest {
    pub issue: UpdateIssue,
}

impl MarkdownOutput for Issue {
    fn to_markdown(&self, _meta: &Meta) -> String {
        let mut output = String::new();
        output.push_str(&format!("## Issue #{}: {}\n\n", self.id, self.subject));

        let mut pairs = vec![
            ("ID", self.id.to_string()),
            ("Subject", self.subject.clone()),
            ("Project", self.project.name.clone()),
            ("Status", self.status.name.clone()),
            ("Priority", self.priority.name.clone()),
        ];

        if let Some(tracker) = &self.tracker {
            pairs.push(("Tracker", tracker.name.clone()));
        }

        if let Some(assignee) = &self.assigned_to {
            pairs.push(("Assignee", assignee.name.clone()));
        }

        if let Some(author) = &self.author {
            pairs.push(("Author", author.name.clone()));
        }

        if let Some(version) = &self.fixed_version {
            pairs.push((
                "Target Version",
                format!("{} (#{})", version.name, version.id),
            ));
        }

        if let Some(parent) = &self.parent {
            pairs.push(("Parent", format!("#{}", parent.id)));
        }

        if let Some(start) = &self.start_date {
            pairs.push(("Start Date", start.clone()));
        }

        if let Some(due) = &self.due_date {
            pairs.push(("Due Date", due.clone()));
        }

        if let Some(done) = self.done_ratio {
            pairs.push(("Done", format!("{}%", done)));
        }

        if let Some(estimated) = self.estimated_hours {
            pairs.push(("Estimated", format!("{:.2}h", estimated)));
        }

        if let Some(spent) = self.spent_hours {
            pairs.push(("Spent", format!("{:.2}h", spent)));
        }

        if let Some(created) = &self.created_on {
            pairs.push(("Created", created.clone()));
        }

        if let Some(updated) = &self.updated_on {
            pairs.push(("Updated", updated.clone()));
        }

        let pairs_ref: Vec<(&str, String)> = pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        output.push_str(&markdown_kv_table(&pairs_ref));

        // Display custom fields if present
        if let Some(custom_fields) = &self.custom_fields {
            if !custom_fields.is_empty() {
                output.push_str("\n### Custom Fields\n\n");
                let cf_pairs: Vec<(&str, String)> = custom_fields
                    .iter()
                    .map(|cf| (cf.name.as_str(), cf.display_value()))
                    .collect();
                output.push_str(&markdown_kv_table(&cf_pairs));
            }
        }

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                output.push_str("\n### Description\n\n");
                output.push_str(desc);
                output.push('\n');
            }
        }

        if let Some(relations) = &self.relations {
            if !relations.is_empty() {
                output.push_str("\n### Relations\n\n");
                for r in relations {
                    output.push_str(&format!(
                        "- {} (relation #{})\n",
                        r.describe_from(self.id),
                        r.id
                    ));
                }
            }
        }

        if let Some(journals) = &self.journals {
            let notes: Vec<&Journal> = journals
                .iter()
                .filter(|j| j.notes.as_deref().map(|n| !n.is_empty()).unwrap_or(false))
                .collect();
            if !notes.is_empty() {
                output.push_str("\n### Comments\n\n");
                for j in notes {
                    output.push_str(&format!(
                        "**#{} — {} ({})**\n\n{}\n\n---\n\n",
                        j.id,
                        j.user.name,
                        j.created_on,
                        j.notes.as_deref().unwrap_or("")
                    ));
                }
            }
        }

        if let Some(attachments) = &self.attachments {
            if !attachments.is_empty() {
                output.push_str("\n### Attachments\n\n");
                for a in attachments {
                    let size = a.filesize.map(format_bytes).unwrap_or_else(|| "-".into());
                    output.push_str(&format!(
                        "- **#{}** {} ({}) — `rdm issue attachment download --id {}`\n",
                        a.id, a.filename, size, a.id
                    ));
                }
            }
        }

        output.push_str(&format!(
            "\n*Use `rdm issue update --id {}` to modify this issue*\n",
            self.id
        ));

        output
    }
}

impl MarkdownOutput for IssueList {
    fn to_markdown(&self, meta: &Meta) -> String {
        let mut output = String::new();

        let total = meta.total_count.unwrap_or(self.issues.len() as u32);
        let offset = meta.offset.unwrap_or(0);
        let showing_end = offset + self.issues.len() as u32;

        output.push_str(&format!(
            "## Issues (showing {}-{} of {})\n\n",
            offset + 1,
            showing_end,
            total
        ));

        if self.issues.is_empty() {
            output.push_str("*No issues found*\n");
            return output;
        }

        let headers = &[
            "ID",
            "Subject",
            "Status",
            "Priority",
            "Assignee",
            "Target Version",
            "Updated",
        ];
        let rows: Vec<Vec<String>> = self
            .issues
            .iter()
            .map(|i| {
                vec![
                    i.id.to_string(),
                    truncate(&i.subject, 40),
                    i.status.name.clone(),
                    i.priority.name.clone(),
                    i.assigned_to
                        .as_ref()
                        .map(|u| u.name.clone())
                        .unwrap_or_else(|| "-".to_string()),
                    i.fixed_version
                        .as_ref()
                        .map(|v| v.name.clone())
                        .unwrap_or_else(|| "-".to_string()),
                    i.updated_on.clone().unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect();

        output.push_str(&markdown_table(headers, rows));

        if let Some(hint) = pagination_hint("rdm issue list ", meta) {
            output.push('\n');
            output.push_str(&hint);
            output.push('\n');
        }

        output
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let keep = max_len.saturating_sub(3);
        let truncated: String = s.chars().take(keep).collect();
        format!("{truncated}...")
    }
}

/// Search result from Redmine search API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: u32,
    pub title: String,
    #[serde(rename = "type")]
    pub result_type: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub datetime: Option<String>,
}

/// Search results response from API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    #[serde(default)]
    pub total_count: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::{truncate, ClearableId, UpdateIssue};

    #[test]
    fn clearable_id_parse() {
        assert_eq!(ClearableId::parse("42"), Ok(ClearableId::Set(42)));
        assert_eq!(ClearableId::parse("#42"), Ok(ClearableId::Set(42)));
        assert_eq!(ClearableId::parse("none"), Ok(ClearableId::Clear));
        assert!(ClearableId::parse("abc").is_err());
    }

    #[test]
    fn parent_serializes_id_or_blank() {
        let set = UpdateIssue {
            parent_issue_id: Some(ClearableId::Set(7)),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(set).unwrap(),
            serde_json::json!({"parent_issue_id": 7})
        );
        let clear = UpdateIssue {
            parent_issue_id: Some(ClearableId::Clear),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(clear).unwrap(),
            serde_json::json!({"parent_issue_id": ""})
        );
    }

    #[test]
    fn assignee_serializes_id_or_blank() {
        let set = UpdateIssue {
            assigned_to_id: Some(ClearableId::Set(5)),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(set).unwrap(),
            serde_json::json!({"assigned_to_id": 5})
        );
        let clear = UpdateIssue {
            assigned_to_id: Some(ClearableId::Clear),
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_value(clear).unwrap(),
            serde_json::json!({"assigned_to_id": ""})
        );
    }

    #[test]
    fn truncate_short_string_is_unchanged() {
        assert_eq!(truncate("hello", 40), "hello");
    }

    #[test]
    fn truncate_ascii_adds_ellipsis() {
        assert_eq!(truncate("abcdefghij", 8), "abcde...");
    }

    #[test]
    fn truncate_multibyte_on_boundary_does_not_panic() {
        // Accented letters occupy two bytes each, so a byte-based slice at the
        // truncation boundary would split one and panic. A string longer than
        // max_len must truncate on a char boundary instead.
        let s = "é".repeat(50);
        let out = truncate(&s, 40);
        assert!(out.ends_with("..."));
        assert_eq!(out.chars().count(), 40);
    }
}

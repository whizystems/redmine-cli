//! Project model.

use super::custom_field::CustomField;
use crate::output::{
    markdown::{markdown_kv_table, markdown_table, pagination_hint},
    MarkdownOutput, Meta,
};
use serde::{Deserialize, Serialize};

/// Minimal project reference (used in embedded responses like issues, time entries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRef {
    pub id: u32,
    pub name: String,
}

/// Project from Redmine API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u32,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub parent: Option<ProjectRef>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<u32>,
    #[serde(default)]
    pub is_public: Option<bool>,
    #[serde(default)]
    pub created_on: Option<String>,
    #[serde(default)]
    pub updated_on: Option<String>,
    /// Project custom fields (omitted from output when Redmine returns none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<CustomField>>,
}

/// List of projects from API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectList {
    pub projects: Vec<Project>,
    #[serde(default)]
    pub total_count: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Wrapper for single project response.
#[derive(Debug, Deserialize)]
pub struct ProjectResponse {
    pub project: Project,
}

impl MarkdownOutput for Project {
    fn to_markdown(&self, _meta: &Meta) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "## Project: {} ({})\n\n",
            self.name, self.identifier
        ));

        let mut pairs = vec![
            ("ID", self.id.to_string()),
            ("Name", self.name.clone()),
            ("Identifier", self.identifier.clone()),
        ];

        let parent_str = self
            .parent
            .as_ref()
            .map(|p| format!("{} (#{})", p.name, p.id))
            .unwrap_or_else(|| "-".to_string());
        pairs.push(("Parent", parent_str));

        if let Some(status) = self.status {
            let status_str = match status {
                1 => "Active",
                5 => "Closed",
                9 => "Archived",
                _ => "Unknown",
            };
            pairs.push(("Status", status_str.to_string()));
        }

        if let Some(is_public) = self.is_public {
            pairs.push(("Public", if is_public { "Yes" } else { "No" }.to_string()));
        }

        if let Some(created) = &self.created_on {
            pairs.push(("Created", created.clone()));
        }

        if let Some(updated) = &self.updated_on {
            pairs.push(("Updated", updated.clone()));
        }

        let pairs_ref: Vec<(&str, String)> = pairs.iter().map(|(k, v)| (*k, v.clone())).collect();
        output.push_str(&markdown_kv_table(&pairs_ref));

        if let Some(custom_fields) = &self.custom_fields {
            if !custom_fields.is_empty() {
                output.push_str("\n### Custom Fields\n\n");
                let headers = &["ID", "Name", "Value"];
                let rows: Vec<Vec<String>> = custom_fields
                    .iter()
                    .map(|cf| vec![cf.id.to_string(), cf.name.clone(), cf.display_value()])
                    .collect();
                output.push_str(&markdown_table(headers, rows));
            }
        }

        if let Some(desc) = &self.description {
            if !desc.is_empty() {
                output.push_str("\n### Description\n\n");
                output.push_str(desc);
                output.push('\n');
            }
        }

        output.push_str(&format!(
            "\n*Use `rdm issue list --project {}` to see issues*\n",
            self.identifier
        ));

        output
    }
}

impl MarkdownOutput for ProjectList {
    fn to_markdown(&self, meta: &Meta) -> String {
        let mut output = String::new();

        let total = meta.total_count.unwrap_or(self.projects.len() as u32);
        let offset = meta.offset.unwrap_or(0);
        let showing_end = offset + self.projects.len() as u32;

        output.push_str(&format!(
            "## Projects (showing {}-{} of {})\n\n",
            offset + 1,
            showing_end,
            total
        ));

        if self.projects.is_empty() {
            output.push_str("*No projects found*\n");
            return output;
        }

        let headers = &["ID", "Identifier", "Name", "Parent", "Status"];
        let rows: Vec<Vec<String>> = self
            .projects
            .iter()
            .map(|p| {
                let status = p
                    .status
                    .map(|s| match s {
                        1 => "Active",
                        5 => "Closed",
                        9 => "Archived",
                        _ => "Unknown",
                    })
                    .unwrap_or("-");
                let parent = p
                    .parent
                    .as_ref()
                    .map(|pr| format!("{} (#{})", pr.name, pr.id))
                    .unwrap_or_else(|| "-".to_string());
                vec![
                    p.id.to_string(),
                    p.identifier.clone(),
                    p.name.clone(),
                    parent,
                    status.to_string(),
                ]
            })
            .collect();

        output.push_str(&markdown_table(headers, rows));

        if let Some(hint) = pagination_hint("rdm project list ", meta) {
            output.push('\n');
            output.push_str(&hint);
            output.push('\n');
        }

        output
    }
}

//! Redmine API client implementation with retry/backoff.

use backoff::{future::retry, ExponentialBackoff};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tracing::{debug, warn};

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::models::*;
use urlencoding;

/// Redmine API client.
pub struct RedmineClient {
    client: Client,
    base_url: String,
    api_key: String,
    dry_run: bool,
}

impl RedmineClient {
    /// Create a new Redmine client.
    pub fn new(config: &Config, dry_run: bool) -> Result<Self> {
        let client = Client::builder()
            .use_rustls_tls()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(format!("rdm/{}", env!("CARGO_PKG_VERSION")))
            .gzip(true)
            .build()
            .map_err(|e| AppError::network(format!("Failed to create HTTP client: {}", e)))?;

        let base_url = config.url.trim_end_matches('/').to_string();

        Ok(Self {
            client,
            base_url,
            api_key: config.api_key.clone(),
            dry_run,
        })
    }

    /// Build a request with authentication.
    fn request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        debug!("Building request: {} {}", method, url);
        self.client
            .request(method, &url)
            .header("X-Redmine-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
    }

    /// Execute a request with retry for transient errors.
    async fn execute(&self, request: RequestBuilder) -> Result<Response> {
        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };

        let request = request
            .build()
            .map_err(|e| AppError::network(format!("Failed to build request: {}", e)))?;

        debug!("Executing request: {} {}", request.method(), request.url());

        let client = self.client.clone();
        let method = request.method().clone();
        let url = request.url().clone();
        let headers = request.headers().clone();
        let body = request
            .body()
            .and_then(|b| b.as_bytes().map(|b| b.to_vec()));

        retry(backoff, || async {
            let mut req_builder = client.request(method.clone(), url.clone());
            for (key, value) in headers.iter() {
                req_builder = req_builder.header(key, value);
            }
            if let Some(ref body_bytes) = body {
                req_builder = req_builder.body(body_bytes.clone());
            }

            let response = req_builder.send().await.map_err(|e| {
                if e.is_timeout() || e.is_connect() {
                    warn!("Transient error, will retry: {}", e);
                    backoff::Error::transient(AppError::network(format!("Request failed: {}", e)))
                } else {
                    backoff::Error::permanent(AppError::network(format!("Request failed: {}", e)))
                }
            })?;

            let status = response.status();
            debug!("Response status: {}", status);

            // Retry on 502, 503, 504
            if matches!(
                status,
                StatusCode::BAD_GATEWAY
                    | StatusCode::SERVICE_UNAVAILABLE
                    | StatusCode::GATEWAY_TIMEOUT
            ) {
                warn!("Server error {}, will retry", status);
                return Err(backoff::Error::transient(AppError::api(
                    format!("Server error: {}", status),
                    Some(status.as_u16()),
                )));
            }

            Ok(response)
        })
        .await
    }

    /// Extract Redmine's `{"errors": [...]}` messages, falling back to the raw body.
    fn error_detail(body: &str) -> String {
        #[derive(serde::Deserialize)]
        struct Errors {
            errors: Vec<String>,
        }
        match serde_json::from_str::<Errors>(body) {
            Ok(e) if !e.errors.is_empty() => e.errors.join("; "),
            _ => body.to_string(),
        }
    }

    /// Parse a JSON response.
    async fn parse_json<T: DeserializeOwned>(response: Response) -> Result<T> {
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AppError::network(format!("Failed to read response: {}", e)))?;

        debug!("Response body: {}", body);

        if status == StatusCode::UNAUTHORIZED {
            return Err(AppError::auth_with_hint(
                "Invalid API key or unauthorized",
                "Check your API key with `rdm config show` or set REDMINE_API_KEY.",
            ));
        }

        if status == StatusCode::FORBIDDEN {
            return Err(AppError::auth("Access forbidden - check your permissions"));
        }

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::api("Resource not found", Some(404)));
        }

        if !status.is_success() {
            return Err(AppError::api(
                format!(
                    "API request failed: {} - {}",
                    status,
                    Self::error_detail(&body)
                ),
                Some(status.as_u16()),
            ));
        }

        serde_json::from_str(&body).map_err(|e| {
            AppError::api(
                format!("Failed to parse response: {} - body: {}", e, body),
                None,
            )
        })
    }

    /// Ping the server to check connectivity.
    pub async fn ping(&self) -> Result<PingResponse> {
        if self.dry_run {
            return Ok(PingResponse {
                status: "dry-run".to_string(),
                url: self.base_url.clone(),
            });
        }

        let response = self
            .execute(self.request(Method::GET, "/users/current.json"))
            .await?;
        let status = response.status();

        if status.is_success() {
            Ok(PingResponse {
                status: "ok".to_string(),
                url: self.base_url.clone(),
            })
        } else if status == StatusCode::UNAUTHORIZED {
            Err(AppError::auth_with_hint(
                "Invalid API key",
                "Check your API key with `rma config show`.",
            ))
        } else {
            Err(AppError::api(
                format!("Server returned {}", status),
                Some(status.as_u16()),
            ))
        }
    }

    /// Get the current user.
    pub async fn me(&self) -> Result<CurrentUser> {
        if self.dry_run {
            return Err(AppError::validation(
                "Cannot use --dry-run with 'me' command",
            ));
        }

        let response = self
            .execute(self.request(Method::GET, "/users/current.json"))
            .await?;
        let wrapper: CurrentUserResponse = Self::parse_json(response).await?;
        Ok(wrapper.user)
    }

    /// List users with optional status filter.
    pub async fn list_users(
        &self,
        status: Option<u32>,
        limit: u32,
        offset: u32,
    ) -> Result<crate::cli::user::UserList> {
        if self.dry_run {
            return Ok(crate::cli::user::UserList {
                users: vec![],
                total_count: Some(0),
                offset: Some(offset),
                limit: Some(limit),
            });
        }

        let mut params = vec![format!("limit={}", limit), format!("offset={}", offset)];

        if let Some(s) = status {
            params.push(format!("status={}", s));
        }

        let path = format!("/users.json?{}", params.join("&"));
        let response = self.execute(self.request(Method::GET, &path)).await?;
        Self::parse_json(response).await
    }

    // === Queries ===

    /// List saved (custom) queries, optionally scoped to a query type.
    ///
    /// `query_type` maps to Redmine's `type` param, e.g. `IssueQuery` or
    /// `TimeEntryQuery`. When `None`, Redmine defaults to `IssueQuery`.
    pub async fn list_queries(&self, query_type: Option<&str>) -> Result<QueryList> {
        if self.dry_run {
            return Ok(QueryList {
                queries: vec![],
                total_count: Some(0),
                offset: Some(0),
                limit: None,
            });
        }

        let path = match query_type {
            Some(t) => format!("/queries.json?type={}", urlencoding::encode(t)),
            None => "/queries.json".to_string(),
        };
        let response = self.execute(self.request(Method::GET, &path)).await?;
        Self::parse_json(response).await
    }

    // === Projects ===

    /// List projects.
    pub async fn list_projects(
        &self,
        limit: u32,
        offset: u32,
        custom_fields: &[(u32, String)],
    ) -> Result<ProjectList> {
        if self.dry_run {
            return Ok(ProjectList {
                projects: vec![],
                total_count: Some(0),
                offset: Some(offset),
                limit: Some(limit),
            });
        }

        let mut path = format!("/projects.json?limit={}&offset={}", limit, offset);
        for (cf_id, cf_value) in custom_fields {
            path.push_str(&format!("&cf_{}={}", cf_id, urlencoding::encode(cf_value)));
        }
        let response = self.execute(self.request(Method::GET, &path)).await?;
        Self::parse_json(response).await
    }

    /// Get a project by ID or identifier.
    pub async fn get_project(&self, id_or_identifier: &str) -> Result<Project> {
        if self.dry_run {
            return Err(AppError::validation(
                "Cannot use --dry-run with 'get' command",
            ));
        }

        let path = format!("/projects/{}.json", id_or_identifier);
        let response = self.execute(self.request(Method::GET, &path)).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Project",
                id_or_identifier,
                "Use `rdm project list` to see available projects.",
            ));
        }

        let wrapper: ProjectResponse = Self::parse_json(response).await?;
        Ok(wrapper.project)
    }

    // === Versions ===

    /// List versions available to a project (including shared ones).
    pub async fn list_versions(&self, project: &str) -> Result<VersionList> {
        if self.dry_run {
            return Ok(VersionList {
                versions: vec![],
                total_count: Some(0),
            });
        }

        let path = format!("/projects/{}/versions.json", urlencoding::encode(project));
        let response = self.execute(self.request(Method::GET, &path)).await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Project",
                project,
                "Use `rdm project list` to see available projects.",
            ));
        }

        Self::parse_json(response).await
    }

    // === Issues ===

    /// List issues with optional filters.
    pub async fn list_issues(&self, filters: IssueFilters) -> Result<IssueList> {
        if self.dry_run {
            return Ok(IssueList {
                issues: vec![],
                total_count: Some(0),
                offset: Some(filters.offset),
                limit: Some(filters.limit),
            });
        }

        let mut params = vec![
            format!("limit={}", filters.limit),
            format!("offset={}", filters.offset),
        ];

        // A project-scoped saved query only resolves when the project is in the
        // URL path (Redmine binds `@project` from the path, not the `project_id`
        // param), so route through `/projects/{id}/issues.json` when a project
        // is given and drop the redundant `project_id` param in that case.
        let base = match &filters.project {
            Some(project) => format!("/projects/{}/issues.json", urlencoding::encode(project)),
            None => "/issues.json".to_string(),
        };

        if let Some(status) = &filters.status {
            params.push(format!("status_id={}", status));
        }
        if let Some(assigned_to) = &filters.assigned_to {
            params.push(format!("assigned_to_id={}", assigned_to));
        }
        if let Some(author) = &filters.author {
            params.push(format!("author_id={}", author));
        }
        if let Some(tracker) = &filters.tracker {
            params.push(format!("tracker_id={}", tracker));
        }
        if !filters.fixed_versions.is_empty() {
            let ids: Vec<String> = filters
                .fixed_versions
                .iter()
                .map(|v| v.to_string())
                .collect();
            params.push(format!(
                "fixed_version_id={}",
                urlencoding::encode(&ids.join("|"))
            ));
        }
        if let Some(subject) = &filters.subject {
            params.push(format!("subject={}", urlencoding::encode(subject)));
        }
        if let Some(query_id) = filters.query_id {
            params.push(format!("query_id={}", query_id));
        }
        if filters.include_relations {
            params.push("include=relations".to_string());
        }
        // Add custom field filters
        for (cf_id, cf_value) in &filters.custom_fields {
            params.push(format!("cf_{}={}", cf_id, urlencoding::encode(cf_value)));
        }

        let path = format!("{}?{}", base, params.join("&"));
        let response = self.execute(self.request(Method::GET, &path)).await?;
        Self::parse_json(response).await
    }

    /// Get an issue by ID.
    pub async fn get_issue(&self, id: u32) -> Result<Issue> {
        if self.dry_run {
            return Err(AppError::validation(
                "Cannot use --dry-run with 'get' command",
            ));
        }

        let path = format!("/issues/{}.json?include=journals,attachments,relations", id);
        let response = self.execute(self.request(Method::GET, &path)).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Issue",
                id.to_string(),
                "Use `rdm issue list` to find available issues.",
            ));
        }

        let wrapper: IssueResponse = Self::parse_json(response).await?;
        Ok(wrapper.issue)
    }

    /// Create a new issue.
    pub async fn create_issue(&self, issue: NewIssue) -> Result<Issue> {
        if self.dry_run {
            let body = serde_json::to_value(&NewIssueRequest { issue })?;
            return Err(AppError::dry_run("POST", "/issues.json", Some(body)));
        }

        let request = self
            .request(Method::POST, "/issues.json")
            .json(&NewIssueRequest { issue });
        let response = self.execute(request).await?;
        let wrapper: IssueResponse = Self::parse_json(response).await?;
        Ok(wrapper.issue)
    }

    /// Update an issue.
    pub async fn update_issue(&self, id: u32, update: UpdateIssue) -> Result<()> {
        if self.dry_run {
            let body = serde_json::to_value(&UpdateIssueRequest { issue: update })?;
            return Err(AppError::dry_run(
                "PUT",
                format!("/issues/{}.json", id),
                Some(body),
            ));
        }

        let tracker_id = update.tracker_id;
        let path = format!("/issues/{}.json", id);
        let request = self
            .request(Method::PUT, &path)
            .json(&UpdateIssueRequest { issue: update });
        let response = self.execute(request).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Issue",
                id.to_string(),
                "Use `rdm issue list` to find available issues.",
            ));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let message = format!("Failed to update issue: {}", Self::error_detail(&body));
            // Redmine rejects trackers not enabled for the issue's project with a 422.
            return Err(match tracker_id {
                Some(tracker) if status == StatusCode::UNPROCESSABLE_ENTITY => {
                    AppError::api_with_hint(
                        message,
                        Some(422),
                        format!(
                            "Tracker {} may not be enabled for this issue's project. Enable it in the project settings or pick another tracker.",
                            tracker
                        ),
                    )
                }
                _ => AppError::api(message, Some(status.as_u16())),
            });
        }

        Ok(())
    }

    /// Search issues using Redmine's search endpoint.
    /// Returns matching issues by fetching full issue data for each search result.
    pub async fn search_issues(
        &self,
        query: &str,
        project: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<IssueList> {
        if self.dry_run {
            return Ok(IssueList {
                issues: vec![],
                total_count: Some(0),
                offset: Some(offset),
                limit: Some(limit),
            });
        }

        let params = [
            format!("q={}", urlencoding::encode(query)),
            "issues=1".to_string(),
            format!("limit={}", limit),
            format!("offset={}", offset),
        ];

        if let Some(project_id) = project {
            // Scope search to a specific project
            let path = format!(
                "/projects/{}/search.json?{}",
                urlencoding::encode(project_id),
                params.join("&")
            );
            let response = self.execute(self.request(Method::GET, &path)).await?;
            let search_results: SearchResults = Self::parse_json(response).await?;
            return self.fetch_issues_from_search(search_results).await;
        }

        let path = format!("/search.json?{}", params.join("&"));
        let response = self.execute(self.request(Method::GET, &path)).await?;
        let search_results: SearchResults = Self::parse_json(response).await?;
        self.fetch_issues_from_search(search_results).await
    }

    /// Fetch full issue data for search results.
    async fn fetch_issues_from_search(&self, search_results: SearchResults) -> Result<IssueList> {
        // Filter to only issue results and extract IDs
        let issue_ids: Vec<u32> = search_results
            .results
            .iter()
            .filter(|r| r.result_type == "issue")
            .map(|r| r.id)
            .collect();

        if issue_ids.is_empty() {
            return Ok(IssueList {
                issues: vec![],
                total_count: Some(0),
                offset: search_results.offset,
                limit: search_results.limit,
            });
        }

        // Fetch full issue data for each result
        let mut issues = Vec::new();
        for id in issue_ids {
            match self.get_issue(id).await {
                Ok(issue) => issues.push(issue),
                Err(e) => {
                    debug!("Skipping inaccessible issue #{}: {}", id, e);
                    continue;
                }
            }
        }

        Ok(IssueList {
            issues,
            total_count: search_results.total_count,
            offset: search_results.offset,
            limit: search_results.limit,
        })
    }

    // === Relations ===

    /// List relations of an issue.
    pub async fn list_relations(&self, issue_id: u32) -> Result<Vec<Relation>> {
        if self.dry_run {
            return Ok(vec![]);
        }

        let path = format!("/issues/{}/relations.json", issue_id);
        let response = self.execute(self.request(Method::GET, &path)).await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Issue",
                issue_id.to_string(),
                "Use `rdm issue list` to find available issues.",
            ));
        }

        let wrapper: RelationsResponse = Self::parse_json(response).await?;
        Ok(wrapper.relations)
    }

    /// Create a relation from `issue_id` to another issue.
    pub async fn create_relation(&self, issue_id: u32, relation: NewRelation) -> Result<Relation> {
        let body = NewRelationRequest { relation };
        if self.dry_run {
            let body = serde_json::to_value(&body)?;
            return Err(AppError::dry_run(
                "POST",
                format!("/issues/{}/relations.json", issue_id),
                Some(body),
            ));
        }

        let path = format!("/issues/{}/relations.json", issue_id);
        let request = self.request(Method::POST, &path).json(&body);
        let response = self.execute(request).await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Issue",
                issue_id.to_string(),
                "Use `rdm issue list` to find available issues.",
            ));
        }

        let wrapper: RelationResponse = Self::parse_json(response).await?;
        Ok(wrapper.relation)
    }

    /// Delete a relation by ID.
    pub async fn delete_relation(&self, id: u32) -> Result<()> {
        if self.dry_run {
            return Err(AppError::dry_run(
                "DELETE",
                format!("/relations/{}.json", id),
                None,
            ));
        }

        let path = format!("/relations/{}.json", id);
        let response = self.execute(self.request(Method::DELETE, &path)).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Relation",
                id.to_string(),
                "Use `rdm issue relation list --id <ISSUE>` to find relations.",
            ));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::api(
                format!("Failed to delete relation: {}", Self::error_detail(&body)),
                Some(status.as_u16()),
            ));
        }

        Ok(())
    }

    // === Attachments ===

    /// Get attachment metadata by ID.
    pub async fn get_attachment(&self, id: u32) -> Result<Attachment> {
        if self.dry_run {
            return Err(AppError::validation(
                "Cannot use --dry-run with attachment commands",
            ));
        }

        let path = format!("/attachments/{}.json", id);
        let response = self.execute(self.request(Method::GET, &path)).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Attachment",
                id.to_string(),
                "Use `rdm issue attachment list --issue-id <ID>` to find attachments.",
            ));
        }

        let wrapper: AttachmentResponse = Self::parse_json(response).await?;
        Ok(wrapper.attachment)
    }

    /// Upload a file and return the upload token.
    pub async fn upload_file(&self, bytes: Vec<u8>, filename: &str) -> Result<String> {
        if self.dry_run {
            return Err(AppError::validation(
                "Cannot use --dry-run with attachment upload",
            ));
        }

        let url = format!(
            "{}/uploads.json?filename={}",
            self.base_url,
            urlencoding::encode(filename)
        );
        let response = self
            .execute(
                self.client
                    .post(&url)
                    .header("X-Redmine-API-Key", &self.api_key)
                    .header("Content-Type", "application/octet-stream")
                    .body(bytes),
            )
            .await?;

        let wrapper: UploadResponse = Self::parse_json(response).await?;
        Ok(wrapper.upload.token)
    }

    /// Download the content of an attachment URL to bytes.
    pub async fn download_attachment(&self, content_url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(content_url)
            .header("X-Redmine-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| AppError::network(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::api(
                format!("Download failed: {}", response.status()),
                Some(response.status().as_u16()),
            ));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| AppError::network(format!("Failed to read download: {}", e)))
    }

    // === Time Entries ===

    /// List time entry activities.
    pub async fn list_activities(&self) -> Result<ActivityList> {
        if self.dry_run {
            return Ok(ActivityList {
                time_entry_activities: vec![],
            });
        }

        let response = self
            .execute(self.request(Method::GET, "/enumerations/time_entry_activities.json"))
            .await?;
        Self::parse_json(response).await
    }

    /// List time entries with optional filters.
    pub async fn list_time_entries(&self, filters: TimeEntryFilters) -> Result<TimeEntryList> {
        if self.dry_run {
            return Ok(TimeEntryList {
                time_entries: vec![],
                total_count: Some(0),
                offset: Some(filters.offset),
                limit: Some(filters.limit),
            });
        }

        let mut params = vec![
            format!("limit={}", filters.limit),
            format!("offset={}", filters.offset),
        ];

        // Route through the project path so project-scoped saved queries resolve
        // (see `list_issues` for why); drop the redundant `project_id` param then.
        let base = match &filters.project {
            Some(project) => format!(
                "/projects/{}/time_entries.json",
                urlencoding::encode(project)
            ),
            None => "/time_entries.json".to_string(),
        };

        if let Some(issue) = &filters.issue {
            params.push(format!("issue_id={}", issue));
        }
        if let Some(user) = &filters.user {
            params.push(format!("user_id={}", user));
        }
        if let Some(from) = &filters.from {
            params.push(format!("from={}", from));
        }
        if let Some(to) = &filters.to {
            params.push(format!("to={}", to));
        }
        if let Some(query_id) = filters.query_id {
            params.push(format!("query_id={}", query_id));
        }
        // Add custom field filters
        for (cf_id, cf_value) in &filters.custom_fields {
            params.push(format!("cf_{}={}", cf_id, urlencoding::encode(cf_value)));
        }

        let path = format!("{}?{}", base, params.join("&"));
        let response = self.execute(self.request(Method::GET, &path)).await?;
        Self::parse_json(response).await
    }

    /// Get a time entry by ID.
    pub async fn get_time_entry(&self, id: u32) -> Result<TimeEntry> {
        if self.dry_run {
            return Err(AppError::validation(
                "Cannot use --dry-run with 'get' command",
            ));
        }

        let path = format!("/time_entries/{}.json", id);
        let response = self.execute(self.request(Method::GET, &path)).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Time entry",
                id.to_string(),
                "Use `rdm time list` to find available time entries.",
            ));
        }

        let wrapper: TimeEntryResponse = Self::parse_json(response).await?;
        Ok(wrapper.time_entry)
    }

    /// Create a new time entry.
    pub async fn create_time_entry(&self, entry: NewTimeEntry) -> Result<TimeEntry> {
        if self.dry_run {
            let body = serde_json::to_value(&NewTimeEntryRequest { time_entry: entry })?;
            return Err(AppError::dry_run("POST", "/time_entries.json", Some(body)));
        }

        let request = self
            .request(Method::POST, "/time_entries.json")
            .json(&NewTimeEntryRequest { time_entry: entry });
        let response = self.execute(request).await?;
        let wrapper: TimeEntryResponse = Self::parse_json(response).await?;
        Ok(wrapper.time_entry)
    }

    /// Update a time entry.
    pub async fn update_time_entry(&self, id: u32, update: UpdateTimeEntry) -> Result<TimeEntry> {
        if self.dry_run {
            let body = serde_json::to_value(&UpdateTimeEntryRequest { time_entry: update })?;
            return Err(AppError::dry_run(
                "PUT",
                format!("/time_entries/{}.json", id),
                Some(body),
            ));
        }

        let path = format!("/time_entries/{}.json", id);
        let request = self
            .request(Method::PUT, &path)
            .json(&UpdateTimeEntryRequest { time_entry: update });
        let response = self.execute(request).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Time entry",
                id.to_string(),
                "Use `rdm time list` to find available time entries.",
            ));
        }

        // Fetch the updated entry to return it
        self.get_time_entry(id).await
    }

    /// Delete a time entry.
    pub async fn delete_time_entry(&self, id: u32) -> Result<()> {
        if self.dry_run {
            return Err(AppError::dry_run(
                "DELETE",
                format!("/time_entries/{}.json", id),
                None,
            ));
        }

        let path = format!("/time_entries/{}.json", id);
        let response = self.execute(self.request(Method::DELETE, &path)).await?;
        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(AppError::not_found_with_hint(
                "Time entry",
                id.to_string(),
                "Use `rdm time list` to find available time entries.",
            ));
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::api(
                format!("Failed to delete time entry: {}", Self::error_detail(&body)),
                Some(status.as_u16()),
            ));
        }

        Ok(())
    }
}

/// Ping response.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PingResponse {
    pub status: String,
    pub url: String,
}

impl crate::output::MarkdownOutput for PingResponse {
    fn to_markdown(&self, _meta: &crate::output::Meta) -> String {
        format!(
            "## Connection Status\n\n- **Status**: {}\n- **URL**: {}\n",
            self.status, self.url
        )
    }
}

/// Issue list filters.
#[derive(Debug, Clone, Default)]
pub struct IssueFilters {
    pub project: Option<String>,
    pub status: Option<String>,
    pub assigned_to: Option<String>,
    pub author: Option<String>,
    pub tracker: Option<String>,
    pub fixed_versions: Vec<u32>,
    pub subject: Option<String>,
    pub query_id: Option<u32>,
    pub custom_fields: Vec<(u32, String)>,
    /// Ask Redmine to embed each issue's relations (`include=relations`).
    pub include_relations: bool,
    pub limit: u32,
    pub offset: u32,
}

impl IssueFilters {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            limit: 25,
            offset: 0,
            ..Default::default()
        }
    }
}

/// Time entry list filters.
#[derive(Debug, Clone, Default)]
pub struct TimeEntryFilters {
    pub project: Option<String>,
    pub issue: Option<u32>,
    pub user: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub query_id: Option<u32>,
    pub custom_fields: Vec<(u32, String)>,
    pub limit: u32,
    pub offset: u32,
}

impl TimeEntryFilters {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            limit: 25,
            offset: 0,
            ..Default::default()
        }
    }
}

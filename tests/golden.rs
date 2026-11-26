//! Golden JSON output tests.
//!
//! These tests verify that the JSON output schema remains stable across changes.
//! They test the envelope structure: `{"ok": true/false, "data": {...}, "meta": {...}, "error": {...}}`

mod common;

use assert_cmd::Command;
use common::*;
use serde_json::Value;

fn get_binary() -> Command {
    Command::cargo_bin("rdm").unwrap()
}

/// Helper to run a command and parse JSON output.
fn run_json_command(cmd: &mut Command) -> (bool, Value) {
    let output = cmd.output().expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Failed to parse JSON: {}\nOutput: {}", e, stdout));
    (output.status.success(), json)
}

// ============================================================================
// Envelope Structure Tests
// ============================================================================

/// Verify the basic envelope structure for success responses.
fn assert_success_envelope(json: &Value) {
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(json["ok"].as_bool().unwrap(), "ok should be true");
    assert!(json.get("data").is_some(), "data field should exist");
    assert!(json.get("meta").is_some(), "meta field should exist");
    assert!(
        json.get("error").is_none() || json["error"].is_null(),
        "error should be null/absent on success"
    );
}

/// Verify the basic envelope structure for error responses.
fn assert_error_envelope(json: &Value) {
    assert!(json.is_object(), "Response should be a JSON object");
    assert!(!json["ok"].as_bool().unwrap(), "ok should be false");
    assert!(
        json.get("data").is_none() || json["data"].is_null(),
        "data should be null/absent on error"
    );
    assert!(json.get("meta").is_some(), "meta field should exist");
    assert!(json.get("error").is_some(), "error field should exist");

    let error = &json["error"];
    assert!(error["code"].is_string(), "error.code should be a string");
    assert!(
        error["message"].is_string(),
        "error.message should be a string"
    );
}

/// Verify pagination metadata structure.
fn assert_paginated_meta(json: &Value) {
    let meta = &json["meta"];
    assert!(meta.is_object(), "meta should be an object");
    // Pagination fields may be present for list commands
    if meta.get("total_count").is_some() {
        assert!(
            meta["total_count"].is_number(),
            "meta.total_count should be a number"
        );
    }
    if meta.get("limit").is_some() {
        assert!(meta["limit"].is_number(), "meta.limit should be a number");
    }
    if meta.get("offset").is_some() {
        assert!(meta["offset"].is_number(), "meta.offset should be a number");
    }
}

// ============================================================================
// Golden Tests: `rdm me --format json`
// ============================================================================

#[tokio::test]
async fn golden_me_json_envelope_structure() {
    let server = start_mock_server().await;
    mock_current_user().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("me");

    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "Command should succeed");
    assert_success_envelope(&json);
}

#[tokio::test]
async fn golden_me_json_data_fields() {
    let server = start_mock_server().await;
    mock_current_user().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("me");

    let (_, json) = run_json_command(&mut cmd);
    let data = &json["data"];

    // Required fields for user data
    assert!(data["id"].is_number(), "data.id should be a number");
    assert!(data["login"].is_string(), "data.login should be a string");
    assert!(
        data["firstname"].is_string(),
        "data.firstname should be a string"
    );
    assert!(
        data["lastname"].is_string(),
        "data.lastname should be a string"
    );
    assert!(data["mail"].is_string(), "data.mail should be a string");
    assert!(data["admin"].is_boolean(), "data.admin should be a boolean");
    assert!(
        data["created_on"].is_string(),
        "data.created_on should be a string"
    );
}

// ============================================================================
// Golden Tests: `rdm project list --format json`
// ============================================================================

#[tokio::test]
async fn golden_project_list_json_envelope_structure() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "list"]);

    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "Command should succeed");
    assert_success_envelope(&json);
    assert_paginated_meta(&json);
}

#[tokio::test]
async fn golden_project_list_json_data_structure() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let data = &json["data"];

    // ProjectList has a "projects" field containing the array
    assert!(data.is_object(), "data should be an object");
    assert!(
        data["projects"].is_array(),
        "data.projects should be an array"
    );
    assert!(
        !data["projects"].as_array().unwrap().is_empty(),
        "data.projects should not be empty"
    );
}

#[tokio::test]
async fn golden_project_list_json_item_fields() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let project = &json["data"]["projects"][0];

    // Required fields for project
    assert!(project["id"].is_number(), "project.id should be a number");
    assert!(
        project["name"].is_string(),
        "project.name should be a string"
    );
    assert!(
        project["identifier"].is_string(),
        "project.identifier should be a string"
    );
    assert!(
        project["status"].is_number(),
        "project.status should be a number"
    );
    assert!(
        project["is_public"].is_boolean(),
        "project.is_public should be a boolean"
    );
    // Parent is surfaced as a {id, name} object
    assert!(
        project["parent"].is_object(),
        "project.parent should be an object"
    );
    assert_eq!(
        project["parent"]["id"], 42,
        "project.parent.id should match mock"
    );
}

#[tokio::test]
async fn golden_project_list_markdown_shows_parent() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["project", "list"]);

    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Parent"),
        "list markdown should have a Parent column, got:\n{stdout}"
    );
    assert!(
        stdout.contains("Parent Project (#42)"),
        "list markdown should render parent as Name (#id), got:\n{stdout}"
    );
}

#[tokio::test]
async fn golden_project_get_markdown_shows_parent() {
    let server = start_mock_server().await;
    mock_project_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["project", "get", "--id", "1"]);

    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Parent Project (#42)"),
        "get markdown should render parent as Name (#id), got:\n{stdout}"
    );
}

#[tokio::test]
async fn golden_project_get_json_includes_custom_fields() {
    let server = start_mock_server().await;
    mock_project_get_with_custom_fields().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "get", "--id", "1"]);

    let (success, json) = run_json_command(&mut cmd);
    assert!(success);
    let cfs = json["data"]["custom_fields"]
        .as_array()
        .expect("data.custom_fields should be an array");
    assert_eq!(cfs.len(), 2);
    assert_eq!(cfs[0]["id"], 7);
    assert_eq!(cfs[0]["name"], "Billable");
    assert_eq!(cfs[0]["value"], "1");
    assert_eq!(cfs[1]["value"], "120");
}

#[tokio::test]
async fn golden_project_get_json_omits_custom_fields_when_absent() {
    let server = start_mock_server().await;
    mock_project_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "get", "--id", "1"]);

    let (success, json) = run_json_command(&mut cmd);
    assert!(success);
    assert!(
        json["data"].get("custom_fields").is_none(),
        "custom_fields should be omitted when the project has none"
    );
}

#[tokio::test]
async fn golden_project_get_markdown_shows_custom_fields() {
    let server = start_mock_server().await;
    mock_project_get_with_custom_fields().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["project", "get", "--id", "1"]);

    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("### Custom Fields"),
        "markdown should have a Custom Fields section, got:\n{stdout}"
    );
    assert!(stdout.contains("| 7 | Billable | 1 |"), "got:\n{stdout}");
    assert!(stdout.contains("| 8 | Estimate | 120 |"), "got:\n{stdout}");
}

#[tokio::test]
async fn golden_project_get_markdown_no_custom_fields_section_when_absent() {
    let server = start_mock_server().await;
    mock_project_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["project", "get", "--id", "1"]);

    let output = cmd.output().expect("command should run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Custom Fields"), "got:\n{stdout}");
}

#[tokio::test]
async fn golden_project_list_cf_filter_and_json_custom_fields() {
    let server = start_mock_server().await;
    mock_projects_list_cf_filter().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "list", "--cf", "7=1"]);

    // The mock only matches when `cf_7=1` is sent, so success proves the filter.
    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "got: {json}");
    let project = &json["data"]["projects"][0];
    assert_eq!(project["identifier"], "billable");
    assert_eq!(project["custom_fields"][0]["name"], "Billable");
}

#[tokio::test]
async fn golden_project_list_rejects_malformed_cf() {
    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", "http://127.0.0.1:9", "--api-key", "test-api-key"])
        .args(["project", "list", "--cf", "billable"]);

    let output = cmd.output().expect("command should run");
    assert!(!output.status.success());
}

// ============================================================================
// Golden Tests: `rdm issue list --format json`
// ============================================================================

#[tokio::test]
async fn golden_issue_list_json_envelope_structure() {
    let server = start_mock_server().await;
    mock_issues_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["issue", "list"]);

    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "Command should succeed");
    assert_success_envelope(&json);
    assert_paginated_meta(&json);
}

#[tokio::test]
async fn golden_issue_list_json_data_structure() {
    let server = start_mock_server().await;
    mock_issues_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["issue", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let data = &json["data"];

    // IssueList has an "issues" field containing the array
    assert!(data.is_object(), "data should be an object");
    assert!(data["issues"].is_array(), "data.issues should be an array");
    assert!(
        !data["issues"].as_array().unwrap().is_empty(),
        "data.issues should not be empty"
    );
}

#[tokio::test]
async fn golden_issue_list_json_item_fields() {
    let server = start_mock_server().await;
    mock_issues_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["issue", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let issue = &json["data"]["issues"][0];

    // Required fields for issue
    assert!(issue["id"].is_number(), "issue.id should be a number");
    assert!(
        issue["subject"].is_string(),
        "issue.subject should be a string"
    );
    assert!(
        issue["project"].is_object(),
        "issue.project should be an object"
    );
    assert!(
        issue["project"]["id"].is_number(),
        "issue.project.id should be a number"
    );
    assert!(
        issue["project"]["name"].is_string(),
        "issue.project.name should be a string"
    );
    assert!(
        issue["status"].is_object(),
        "issue.status should be an object"
    );
    assert!(
        issue["status"]["id"].is_number(),
        "issue.status.id should be a number"
    );
    assert!(
        issue["status"]["name"].is_string(),
        "issue.status.name should be a string"
    );
    assert!(
        issue["priority"].is_object(),
        "issue.priority should be an object"
    );
    assert!(
        issue["author"].is_object(),
        "issue.author should be an object"
    );
}

// ============================================================================
// Golden Tests: `rdm time list --format json`
// ============================================================================

#[tokio::test]
async fn golden_time_list_json_envelope_structure() {
    let server = start_mock_server().await;
    mock_time_entries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["time", "list"]);

    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "Command should succeed");
    assert_success_envelope(&json);
    assert_paginated_meta(&json);
}

#[tokio::test]
async fn golden_time_list_json_data_structure() {
    let server = start_mock_server().await;
    mock_time_entries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["time", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let data = &json["data"];

    // TimeEntryList has a "time_entries" field containing the array
    assert!(data.is_object(), "data should be an object");
    assert!(
        data["time_entries"].is_array(),
        "data.time_entries should be an array"
    );
    assert!(
        !data["time_entries"].as_array().unwrap().is_empty(),
        "data.time_entries should not be empty"
    );
}

#[tokio::test]
async fn golden_time_list_json_item_fields() {
    let server = start_mock_server().await;
    mock_time_entries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["time", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let entry = &json["data"]["time_entries"][0];

    // Required fields for time entry
    assert!(entry["id"].is_number(), "entry.id should be a number");
    assert!(entry["hours"].is_number(), "entry.hours should be a number");
    assert!(
        entry["spent_on"].is_string(),
        "entry.spent_on should be a string"
    );
    assert!(
        entry["activity"].is_object(),
        "entry.activity should be an object"
    );
    assert!(
        entry["activity"]["id"].is_number(),
        "entry.activity.id should be a number"
    );
    assert!(
        entry["activity"]["name"].is_string(),
        "entry.activity.name should be a string"
    );
    assert!(entry["user"].is_object(), "entry.user should be an object");
    assert!(
        entry["user"]["id"].is_number(),
        "entry.user.id should be a number"
    );
}

// ============================================================================
// Golden Tests: Error Cases
// ============================================================================

#[test]
fn golden_error_missing_credentials_json() {
    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .env_remove("REDMINE_URL")
        .env_remove("REDMINE_API_KEY")
        .args(["--format", "json"])
        .arg("ping");

    let output = cmd.output().expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: Value = serde_json::from_str(&stderr).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON from stderr: {}\nOutput: {}",
            e, stderr
        )
    });

    assert!(!output.status.success(), "Command should fail");
    assert_error_envelope(&json);

    let error = &json["error"];
    assert_eq!(error["code"].as_str().unwrap(), "CONFIG_ERROR");
}

#[tokio::test]
async fn golden_error_api_error_json() {
    let server = start_mock_server().await;
    // Mount a mock that returns 401 Unauthorized
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/users/current.json"))
        .respond_with(
            wiremock::ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "errors": ["Invalid API key"]
            })),
        )
        .mount(&server)
        .await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "invalid-key"])
        .args(["--format", "json"])
        .arg("me");

    let output = cmd.output().expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: Value = serde_json::from_str(&stderr).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON from stderr: {}\nOutput: {}",
            e, stderr
        )
    });

    assert!(!output.status.success(), "Command should fail");
    assert_error_envelope(&json);

    let error = &json["error"];
    assert_eq!(error["code"].as_str().unwrap(), "AUTH_ERROR");
}

#[tokio::test]
async fn golden_error_not_found_json() {
    let server = start_mock_server().await;
    // Mount a mock that returns 404 Not Found
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path_regex(r"/issues/\d+\.json"))
        .respond_with(wiremock::ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["issue", "get", "--id", "99999"]);

    let output = cmd.output().expect("Failed to execute command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let json: Value = serde_json::from_str(&stderr).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON from stderr: {}\nOutput: {}",
            e, stderr
        )
    });

    assert!(!output.status.success(), "Command should fail");
    assert_error_envelope(&json);

    let error = &json["error"];
    assert_eq!(error["code"].as_str().unwrap(), "NOT_FOUND");
}

// ============================================================================
// Golden Tests: Meta Pagination
// ============================================================================

#[tokio::test]
async fn golden_meta_pagination_fields() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["project", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let meta = &json["meta"];

    // Verify pagination metadata fields
    assert_eq!(meta["total_count"].as_u64().unwrap(), 1);
    assert_eq!(meta["limit"].as_u64().unwrap(), 25);
    assert_eq!(meta["offset"].as_u64().unwrap(), 0);
    // next_offset should be absent when on last page
    assert!(
        meta.get("next_offset").is_none() || meta["next_offset"].is_null(),
        "next_offset should be absent on last page"
    );
}

// ============================================================================
// Golden Tests: Activities List
// ============================================================================

#[tokio::test]
async fn golden_time_activities_list_json_envelope_structure() {
    let server = start_mock_server().await;
    mock_activities().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["time", "activities", "list"]);

    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "Command should succeed");
    assert_success_envelope(&json);
}

#[tokio::test]
async fn golden_time_activities_list_json_data_structure() {
    let server = start_mock_server().await;
    mock_activities().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["time", "activities", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let data = &json["data"];

    // ActivityList has a "time_entry_activities" field containing the array
    assert!(data.is_object(), "data should be an object");
    assert!(
        data["time_entry_activities"].is_array(),
        "data.time_entry_activities should be an array"
    );
    assert!(
        !data["time_entry_activities"].as_array().unwrap().is_empty(),
        "data.time_entry_activities should not be empty"
    );
}

#[tokio::test]
async fn golden_time_activities_list_json_item_fields() {
    let server = start_mock_server().await;
    mock_activities().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .args(["time", "activities", "list"]);

    let (_, json) = run_json_command(&mut cmd);
    let activity = &json["data"]["time_entry_activities"][0];

    assert!(activity["id"].is_number(), "activity.id should be a number");
    assert!(
        activity["name"].is_string(),
        "activity.name should be a string"
    );
    assert!(
        activity["is_default"].is_boolean(),
        "activity.is_default should be a boolean"
    );
}

// ============================================================================
// Golden Tests: Ping Command
// ============================================================================

#[tokio::test]
async fn golden_ping_json_envelope_structure() {
    let server = start_mock_server().await;
    mock_current_user().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("ping");

    let (success, json) = run_json_command(&mut cmd);
    assert!(success, "Command should succeed");
    assert_success_envelope(&json);
}

#[tokio::test]
async fn golden_ping_json_data_fields() {
    let server = start_mock_server().await;
    mock_current_user().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("ping");

    let (_, json) = run_json_command(&mut cmd);
    let data = &json["data"];

    // Ping should return connection status
    assert!(data["status"].is_string(), "data.status should be a string");
    assert_eq!(data["status"].as_str().unwrap(), "ok");
}

//! Integration tests for the rdm CLI.

mod common;

use assert_cmd::Command;
use common::*;
use predicates::prelude::*;
use wiremock::ResponseTemplate;

fn get_binary() -> Command {
    Command::cargo_bin("rdm").unwrap()
}

// ============================================================================
// Project Commands
// ============================================================================

#[tokio::test]
async fn test_project_list() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("project")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Project"));
}

#[tokio::test]
async fn test_project_list_json() {
    let server = start_mock_server().await;
    mock_projects_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("project")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("\"name\": \"Test Project\""));
}

#[tokio::test]
async fn test_project_get() {
    let server = start_mock_server().await;
    mock_project_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("project")
        .arg("get")
        .args(["--identifier", "test-project"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Project"));
}

// ============================================================================
// Issue Commands
// ============================================================================

#[tokio::test]
async fn test_issue_list() {
    let server = start_mock_server().await;
    mock_issues_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Issue"))
        .stdout(predicate::str::contains("123"));
}

#[tokio::test]
async fn test_issue_list_json() {
    let server = start_mock_server().await;
    mock_issues_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("issue")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("\"subject\": \"Test Issue\""));
}

#[tokio::test]
async fn test_issue_get() {
    let server = start_mock_server().await;
    mock_issue_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("get")
        .args(["--id", "123"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Issue"))
        .stdout(predicate::str::contains("#123"));
}

#[tokio::test]
async fn test_issue_list_query_id_project_scoped() {
    // A --query-id with --project must route through /projects/{id}/issues.json.
    let server = start_mock_server().await;
    mock_project_issues_query().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("list")
        .args(["--query-id", "5", "--project", "10"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Test Issue"));
}

#[tokio::test]
async fn test_issue_get_shows_target_version() {
    let server = start_mock_server().await;
    mock_issue_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("get")
        .args(["--id", "123"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Target Version"))
        .stdout(predicate::str::contains("Milestone 1 (#116)"));
}

#[tokio::test]
async fn test_issue_get_json_includes_fixed_version() {
    let server = start_mock_server().await;
    mock_issue_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("issue")
        .arg("get")
        .args(["--id", "123"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"fixed_version\""))
        .stdout(predicate::str::contains("\"name\": \"Milestone 1\""));
}

#[tokio::test]
async fn test_issue_list_version_filter_by_name_and_id() {
    // Names resolve via the project's versions, mix with IDs, and join with `|`.
    let server = start_mock_server().await;
    mock_versions_list().mount(&server).await;
    mock_project_issues_by_version().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("list")
        .args(["--project", "10", "--version", "milestone 1,117"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Target Version"))
        .stdout(predicate::str::contains("Milestone 1"));
}

#[tokio::test]
async fn test_issue_list_version_name_requires_project() {
    let server = start_mock_server().await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("list")
        .args(["--version", "Milestone 1"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("without a project"));
}

#[tokio::test]
async fn test_issue_list_version_unknown_name() {
    let server = start_mock_server().await;
    mock_versions_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("list")
        .args(["--project", "10", "--version", "Milestone 9"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Milestone 9"));
}

#[tokio::test]
async fn test_issue_update_version_by_name() {
    // A name resolves within the issue's project, then is sent as fixed_version_id.
    let server = start_mock_server().await;
    mock_issue_get().mount(&server).await;
    mock_versions_list().mount(&server).await;
    mock_issue_update_version(117).mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("update")
        .args(["--id", "123", "--version", "Milestone 2"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Issue #123 has been updated"));
}

#[tokio::test]
async fn test_issue_update_version_by_id() {
    let server = start_mock_server().await;
    mock_issue_update_version(116).mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("issue")
        .arg("update")
        .args(["--id", "123", "--version", "116"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Issue #123 has been updated"));
}

// ============================================================================
// Version Commands
// ============================================================================

#[tokio::test]
async fn test_version_list() {
    let server = start_mock_server().await;
    mock_versions_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("version")
        .arg("list")
        .args(["--project", "10"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Milestone 1"))
        .stdout(predicate::str::contains("Milestone 2"))
        .stdout(predicate::str::contains("116"))
        .stdout(predicate::str::contains("2024-03-01"))
        .stdout(predicate::str::contains("closed"));
}

// ============================================================================
// Query Commands
// ============================================================================

#[tokio::test]
async fn test_query_list() {
    let server = start_mock_server().await;
    mock_queries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("query")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Global Query"))
        .stdout(predicate::str::contains("Project Query"))
        .stdout(predicate::str::contains("global"))
        .stdout(predicate::str::contains("project 10"));
}

#[tokio::test]
async fn test_query_list_type_time_json() {
    let server = start_mock_server().await;
    mock_queries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("query")
        .arg("list")
        .args(["--type", "time"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("\"name\": \"Global Query\""));
}

// ============================================================================
// Time Entry Commands
// ============================================================================

#[tokio::test]
async fn test_time_list() {
    let server = start_mock_server().await;
    mock_time_entries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("time")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("456"))
        .stdout(predicate::str::contains("2.50"))
        .stdout(predicate::str::contains("Development"));
}

#[tokio::test]
async fn test_time_list_json() {
    let server = start_mock_server().await;
    mock_time_entries_list().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .args(["--format", "json"])
        .arg("time")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("\"hours\": 2.5"));
}

#[tokio::test]
async fn test_time_list_query_id_project_scoped() {
    // A --query-id with --project must route through /projects/{id}/time_entries.json.
    let server = start_mock_server().await;
    mock_project_time_entries_query().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("time")
        .arg("list")
        .args(["--query-id", "112", "--project", "113"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("456"))
        .stdout(predicate::str::contains("Development"));
}

#[tokio::test]
async fn test_time_get() {
    let server = start_mock_server().await;
    mock_time_entry_get().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("time")
        .arg("get")
        .args(["--id", "456"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Time Entry #456"))
        .stdout(predicate::str::contains("2.50"));
}

#[tokio::test]
async fn test_time_delete() {
    let server = start_mock_server().await;
    mock_time_entry_delete().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("time")
        .arg("delete")
        .args(["--id", "456"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Time Entry Deleted"));
}

#[tokio::test]
async fn test_time_activities_list() {
    let server = start_mock_server().await;
    mock_activities().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("time")
        .arg("activities")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Development"))
        .stdout(predicate::str::contains("Design"))
        .stdout(predicate::str::contains("Testing"));
}

// ============================================================================
// Me Command
// ============================================================================

#[tokio::test]
async fn test_me() {
    let server = start_mock_server().await;
    mock_current_user().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("me");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("testuser"))
        .stdout(predicate::str::contains("Test User"));
}

// ============================================================================
// Ping Command
// ============================================================================

#[tokio::test]
async fn test_ping() {
    let server = start_mock_server().await;
    mock_current_user().mount(&server).await;

    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"])
        .arg("ping");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Connection Status"))
        .stdout(predicate::str::contains("ok"));
}

// ============================================================================
// Error Handling
// ============================================================================

#[test]
fn test_missing_credentials() {
    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .env_remove("REDMINE_URL")
        .env_remove("REDMINE_API_KEY")
        .arg("ping");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No Redmine credentials"));
}

#[test]
fn test_help() {
    let mut cmd = get_binary();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Agent-first Redmine CLI"))
        .stdout(predicate::str::contains("ping"))
        .stdout(predicate::str::contains("issue"))
        .stdout(predicate::str::contains("time"));
}

#[test]
fn test_version() {
    let mut cmd = get_binary();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("rdm"));
}

// ============================================================================
// Profile Commands
// ============================================================================

#[test]
fn test_profile_list_empty() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = get_binary();
    cmd.env("APPDATA", temp.path())
        .env("LOCALAPPDATA", temp.path())
        .env_remove("REDMINE_URL")
        .env_remove("REDMINE_API_KEY")
        .arg("profile")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No profiles"));
}

// ============================================================================
// Relations and parent
// ============================================================================

fn rdm(server: &wiremock::MockServer) -> Command {
    let mut cmd = get_binary();
    cmd.env("APPDATA", std::env::temp_dir())
        .env("LOCALAPPDATA", std::env::temp_dir())
        .args(["--url", &server.uri(), "--api-key", "test-api-key"]);
    cmd
}

fn json_stdout(cmd: &mut Command) -> serde_json::Value {
    let out = cmd.output().expect("run rdm");
    assert!(out.status.success(), "rdm failed: {:?}", out);
    serde_json::from_slice(&out.stdout).expect("json stdout")
}

#[tokio::test]
async fn test_issue_get_json_includes_parent_and_relations() {
    let server = start_mock_server().await;
    mock_issue_get_with_relations().mount(&server).await;

    let json = json_stdout(rdm(&server).args(["--format", "json", "issue", "get", "--id", "123"]));
    let issue = &json["data"];
    assert_eq!(issue["parent"], serde_json::json!({"id": 100}));
    assert_eq!(
        issue["relations"][0],
        serde_json::json!({
            "id": 1, "issue_id": 123, "issue_to_id": 124,
            "relation_type": "blocks", "delay": null
        })
    );
    assert_eq!(issue["relations"][1]["delay"], 2);
}

#[tokio::test]
async fn test_issue_get_markdown_shows_parent_and_relations() {
    let server = start_mock_server().await;
    mock_issue_get_with_relations().mount(&server).await;

    rdm(&server)
        .args(["issue", "get", "--id", "123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("| Parent | #100 |"))
        .stdout(predicate::str::contains("### Relations"))
        .stdout(predicate::str::contains("- blocks #124 (relation #1)"))
        .stdout(predicate::str::contains(
            "- follows #120 (delay 2d) (relation #2)",
        ));
}

#[tokio::test]
async fn test_issue_list_json_omits_parent_and_relations_when_absent() {
    let server = start_mock_server().await;
    mock_issues_list().mount(&server).await;

    let json = json_stdout(rdm(&server).args(["--format", "json", "issue", "list"]));
    let issue = &json["data"]["issues"][0];
    assert!(issue.get("parent").is_none());
    assert!(issue.get("relations").is_none());
}

#[tokio::test]
async fn test_issue_list_include_relations() {
    let server = start_mock_server().await;
    mock_issues_list_with_relations().mount(&server).await;

    let json = json_stdout(rdm(&server).args([
        "--format",
        "json",
        "issue",
        "list",
        "--include-relations",
    ]));
    let issue = &json["data"]["issues"][0];
    assert_eq!(issue["parent"]["id"], 100);
    assert_eq!(issue["relations"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_issue_relation_list() {
    let server = start_mock_server().await;
    mock_relations_list().mount(&server).await;

    let json = json_stdout(rdm(&server).args([
        "--format", "json", "issue", "relation", "list", "--id", "123",
    ]));
    assert_eq!(json["data"]["issue_id"], 123);
    assert_eq!(json["data"]["relations"][1]["relation_type"], "precedes");

    rdm(&server)
        .args(["issue", "relation", "list", "--id", "123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Relations for Issue #123 (2)"))
        .stdout(predicate::str::contains("blocked by").not())
        .stdout(predicate::str::contains("follows #120"));
}

#[tokio::test]
async fn test_issue_relation_add() {
    let server = start_mock_server().await;
    mock_relation_create(serde_json::json!({
        "relation": {"issue_to_id": 124, "relation_type": "precedes", "delay": 2}
    }))
    .mount(&server)
    .await;

    let json = json_stdout(rdm(&server).args([
        "--format", "json", "issue", "relation", "add", "--id", "123", "--to", "124", "--type",
        "precedes", "--delay", "2",
    ]));
    assert_eq!(json["data"]["relation"]["id"], 9);
}

#[tokio::test]
async fn test_issue_relation_add_snake_case_type() {
    let server = start_mock_server().await;
    mock_relation_create(serde_json::json!({
        "relation": {"issue_to_id": 124, "relation_type": "copied_to"}
    }))
    .mount(&server)
    .await;

    rdm(&server)
        .args([
            "issue",
            "relation",
            "add",
            "--id",
            "123",
            "--to",
            "124",
            "--type",
            "copied_to",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Relation Created"));
}

#[tokio::test]
async fn test_issue_relation_add_rejects_unknown_type() {
    let server = start_mock_server().await;
    rdm(&server)
        .args([
            "issue", "relation", "add", "--id", "123", "--to", "124", "--type", "depends",
        ])
        .assert()
        .failure();
}

#[tokio::test]
async fn test_issue_relation_remove() {
    let server = start_mock_server().await;
    mock_relation_delete().mount(&server).await;

    let json = json_stdout(rdm(&server).args([
        "--format",
        "json",
        "issue",
        "relation",
        "remove",
        "--relation-id",
        "9",
    ]));
    assert_eq!(
        json["data"],
        serde_json::json!({"relation_id": 9, "deleted": true})
    );
}

#[tokio::test]
async fn test_issue_update_parent_set() {
    let server = start_mock_server().await;
    mock_issue_update_parent(serde_json::json!(100))
        .mount(&server)
        .await;

    rdm(&server)
        .args(["issue", "update", "--id", "123", "--parent", "100"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #123 has been updated"));
}

#[tokio::test]
async fn test_issue_update_parent_clear() {
    let server = start_mock_server().await;
    mock_issue_update_parent(serde_json::json!(""))
        .mount(&server)
        .await;

    rdm(&server)
        .args(["issue", "update", "--id", "123", "--parent", "none"])
        .assert()
        .success();
}

#[tokio::test]
async fn test_issue_update_assignee_set() {
    let server = start_mock_server().await;
    mock_issue_update(
        serde_json::json!({"assigned_to_id": 5}),
        ResponseTemplate::new(204),
    )
    .mount(&server)
    .await;

    rdm(&server)
        .args(["issue", "update", "--id", "123", "--assigned-to", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #123 has been updated"));
}

#[tokio::test]
async fn test_issue_update_assignee_clear() {
    let server = start_mock_server().await;
    mock_issue_update(
        serde_json::json!({"assigned_to_id": ""}),
        ResponseTemplate::new(204),
    )
    .mount(&server)
    .await;

    rdm(&server)
        .args(["issue", "update", "--id", "123", "--assigned-to", "none"])
        .assert()
        .success();
}

#[tokio::test]
async fn test_issue_update_tracker() {
    let server = start_mock_server().await;
    mock_issue_update(
        serde_json::json!({"tracker_id": 2}),
        ResponseTemplate::new(204),
    )
    .mount(&server)
    .await;

    rdm(&server)
        .args(["issue", "update", "--id", "123", "--tracker", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #123 has been updated"));
}

#[tokio::test]
async fn test_issue_update_tracker_not_enabled() {
    let server = start_mock_server().await;
    mock_issue_update(
        serde_json::json!({"tracker_id": 9}),
        ResponseTemplate::new(422)
            .set_body_json(serde_json::json!({"errors": ["Tracker is not included in the list"]})),
    )
    .mount(&server)
    .await;

    rdm(&server)
        .args(["issue", "update", "--id", "123", "--tracker", "9"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Tracker is not included in the list",
        ))
        .stderr(predicate::str::contains(
            "Tracker 9 may not be enabled for this issue's project",
        ));
}

#[tokio::test]
async fn test_issue_create_with_parent() {
    let server = start_mock_server().await;
    mock_issue_create_with_parent(100).mount(&server).await;

    let json = json_stdout(rdm(&server).args([
        "--format",
        "json",
        "issue",
        "create",
        "--project",
        "1",
        "--subject",
        "Child",
        "--parent",
        "100",
    ]));
    assert_eq!(json["data"]["issue"]["parent"]["id"], 100);
}

//! Common test utilities.

use wiremock::matchers::{body_partial_json, header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Start a mock Redmine server.
pub async fn start_mock_server() -> MockServer {
    MockServer::start().await
}

/// Create a mock for the current user endpoint.
pub fn mock_current_user() -> Mock {
    Mock::given(method("GET"))
        .and(path("/users/current.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {
                "id": 1,
                "login": "testuser",
                "firstname": "Test",
                "lastname": "User",
                "mail": "test@example.com",
                "admin": false,
                "created_on": "2024-01-01T00:00:00Z",
                "last_login_on": "2024-01-15T12:00:00Z"
            }
        })))
}

/// Create a mock for the activities endpoint.
pub fn mock_activities() -> Mock {
    Mock::given(method("GET"))
        .and(path("/enumerations/time_entry_activities.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "time_entry_activities": [
                {"id": 1, "name": "Development", "is_default": true},
                {"id": 2, "name": "Design", "is_default": false},
                {"id": 3, "name": "Testing", "is_default": false}
            ]
        })))
}

/// Create a mock for the projects list endpoint.
pub fn mock_projects_list() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects\.json.*"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "projects": [
                {
                    "id": 1,
                    "name": "Test Project",
                    "identifier": "test-project",
                    "parent": {"id": 42, "name": "Parent Project"},
                    "description": "A test project",
                    "status": 1,
                    "is_public": true,
                    "created_on": "2024-01-01T00:00:00Z",
                    "updated_on": "2024-01-15T12:00:00Z"
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for getting a single project.
pub fn mock_project_get() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects/[^/]+\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "project": {
                "id": 1,
                "name": "Test Project",
                "identifier": "test-project",
                "parent": {"id": 42, "name": "Parent Project"},
                "description": "A test project",
                "status": 1,
                "is_public": true,
                "created_on": "2024-01-01T00:00:00Z",
                "updated_on": "2024-01-15T12:00:00Z"
            }
        })))
}

/// Create a mock for getting a project that has custom fields.
pub fn mock_project_get_with_custom_fields() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects/[^/]+\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "project": {
                "id": 1,
                "name": "Test Project",
                "identifier": "test-project",
                "status": 1,
                "is_public": true,
                "custom_fields": [
                    {"id": 7, "name": "Billable", "value": "1"},
                    {"id": 8, "name": "Estimate", "value": "120"}
                ]
            }
        })))
}

/// Create a mock for the projects list endpoint filtered by a custom field.
pub fn mock_projects_list_cf_filter() -> Mock {
    Mock::given(method("GET"))
        .and(path("/projects.json"))
        .and(query_param("cf_7", "1"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "projects": [
                {
                    "id": 1,
                    "name": "Billable Project",
                    "identifier": "billable",
                    "status": 1,
                    "custom_fields": [{"id": 7, "name": "Billable", "value": "1"}]
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for the issues list endpoint.
pub fn mock_issues_list() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/issues\.json.*"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issues": [
                {
                    "id": 123,
                    "subject": "Test Issue",
                    "project": {"id": 1, "name": "Test Project", "identifier": "test-project"},
                    "status": {"id": 1, "name": "New"},
                    "priority": {"id": 2, "name": "Normal"},
                    "author": {"id": 1, "name": "Test User"},
                    "created_on": "2024-01-01T00:00:00Z",
                    "updated_on": "2024-01-15T12:00:00Z"
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for a project-scoped issues list run via saved query 5.
///
/// Requires `query_id=5` so the test fails if the CLI drops the parameter.
pub fn mock_project_issues_query() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects/[^/]+/issues\.json.*"))
        .and(query_param("query_id", "5"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issues": [
                {
                    "id": 123,
                    "subject": "Test Issue",
                    "project": {"id": 1, "name": "Test Project", "identifier": "test-project"},
                    "status": {"id": 1, "name": "New"},
                    "priority": {"id": 2, "name": "Normal"},
                    "author": {"id": 1, "name": "Test User"},
                    "created_on": "2024-01-01T00:00:00Z",
                    "updated_on": "2024-01-15T12:00:00Z"
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for the saved queries list endpoint.
pub fn mock_queries_list() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/queries\.json.*"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "queries": [
                {"id": 1, "name": "Global Query", "is_public": true, "project_id": null},
                {"id": 2, "name": "Project Query", "is_public": false, "project_id": 10}
            ],
            "total_count": 2,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for getting a single issue.
pub fn mock_issue_get() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/issues/\d+\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issue": {
                "id": 123,
                "subject": "Test Issue",
                "description": "This is a test issue",
                "project": {"id": 1, "name": "Test Project", "identifier": "test-project"},
                "status": {"id": 1, "name": "New"},
                "priority": {"id": 2, "name": "Normal"},
                "tracker": {"id": 1, "name": "Bug"},
                "author": {"id": 1, "name": "Test User"},
                "fixed_version": {"id": 116, "name": "Milestone 1"},
                "created_on": "2024-01-01T00:00:00Z",
                "updated_on": "2024-01-15T12:00:00Z"
            }
        })))
}

/// Create a mock for a project's versions list.
pub fn mock_versions_list() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects/[^/]+/versions\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "versions": [
                {
                    "id": 116,
                    "name": "Milestone 1",
                    "project": {"id": 10, "name": "Test Project"},
                    "status": "open",
                    "due_date": "2024-03-01",
                    "sharing": "none"
                },
                {
                    "id": 117,
                    "name": "Milestone 2",
                    "project": {"id": 10, "name": "Test Project"},
                    "status": "closed",
                    "due_date": null,
                    "sharing": "none"
                }
            ],
            "total_count": 2
        })))
}

/// Create a mock for a project-scoped issues list filtered by versions 116 and 117.
///
/// Requires `fixed_version_id=116|117` so the test fails if resolution or joining breaks.
pub fn mock_project_issues_by_version() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects/[^/]+/issues\.json.*"))
        .and(query_param("fixed_version_id", "116|117"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issues": [
                {
                    "id": 123,
                    "subject": "Test Issue",
                    "project": {"id": 10, "name": "Test Project"},
                    "status": {"id": 1, "name": "New"},
                    "priority": {"id": 2, "name": "Normal"},
                    "fixed_version": {"id": 116, "name": "Milestone 1"},
                    "updated_on": "2024-01-15T12:00:00Z"
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for updating an issue; requires `fields` within the body's `issue`.
pub fn mock_issue_update(fields: serde_json::Value, response: ResponseTemplate) -> Mock {
    Mock::given(method("PUT"))
        .and(path_regex(r"/issues/\d+\.json"))
        .and(body_partial_json(serde_json::json!({ "issue": fields })))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(response)
}

/// Create a mock for updating an issue; requires `fixed_version_id` in the body.
pub fn mock_issue_update_version(version_id: u32) -> Mock {
    mock_issue_update(
        serde_json::json!({"fixed_version_id": version_id}),
        ResponseTemplate::new(204),
    )
}

/// Create a mock for time entries list endpoint.
pub fn mock_time_entries_list() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/time_entries\.json.*"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "time_entries": [
                {
                    "id": 456,
                    "hours": 2.5,
                    "comments": "Test comment",
                    "spent_on": "2024-01-15",
                    "activity": {"id": 1, "name": "Development"},
                    "user": {"id": 1, "name": "Test User"},
                    "issue": {"id": 123},
                    "created_on": "2024-01-15T12:00:00Z",
                    "updated_on": "2024-01-15T12:00:00Z"
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for a project-scoped time entries list run via saved query 112.
///
/// Requires `query_id=112` so the test fails if the CLI drops the parameter.
pub fn mock_project_time_entries_query() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/projects/[^/]+/time_entries\.json.*"))
        .and(query_param("query_id", "112"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "time_entries": [
                {
                    "id": 456,
                    "hours": 2.5,
                    "comments": "Test comment",
                    "spent_on": "2024-01-15",
                    "activity": {"id": 1, "name": "Development"},
                    "user": {"id": 1, "name": "Test User"},
                    "issue": {"id": 123},
                    "created_on": "2024-01-15T12:00:00Z",
                    "updated_on": "2024-01-15T12:00:00Z"
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for getting a single time entry.
pub fn mock_time_entry_get() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/time_entries/\d+\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "time_entry": {
                "id": 456,
                "hours": 2.5,
                "comments": "Test comment",
                "spent_on": "2024-01-15",
                "activity": {"id": 1, "name": "Development"},
                "user": {"id": 1, "name": "Test User"},
                "project": {"id": 1, "name": "Test Project", "identifier": "test-project"},
                "issue": {"id": 123},
                "created_on": "2024-01-15T12:00:00Z",
                "updated_on": "2024-01-15T12:00:00Z"
            }
        })))
}

/// Create a mock for creating a time entry.
pub fn mock_time_entry_create() -> Mock {
    Mock::given(method("POST"))
        .and(path("/time_entries.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "time_entry": {
                "id": 789,
                "hours": 1.5,
                "comments": "New entry",
                "spent_on": "2024-01-16",
                "activity": {"id": 1, "name": "Development"},
                "user": {"id": 1, "name": "Test User"},
                "issue": {"id": 123},
                "created_on": "2024-01-16T12:00:00Z",
                "updated_on": "2024-01-16T12:00:00Z"
            }
        })))
}

/// Create a mock for updating a time entry (PUT returns no body, then we GET).
pub fn mock_time_entry_update() -> Mock {
    Mock::given(method("PUT"))
        .and(path_regex(r"/time_entries/\d+\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200))
}

/// Create a mock for deleting a time entry.
pub fn mock_time_entry_delete() -> Mock {
    Mock::given(method("DELETE"))
        .and(path_regex(r"/time_entries/\d+\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200))
}

/// Relations fixture for issue 123: it blocks #124, and #120 precedes it.
fn relations_fixture() -> serde_json::Value {
    serde_json::json!([
        {"id": 1, "issue_id": 123, "issue_to_id": 124, "relation_type": "blocks", "delay": null},
        {"id": 2, "issue_id": 120, "issue_to_id": 123, "relation_type": "precedes", "delay": 2}
    ])
}

/// Create a mock for getting an issue with a parent and relations.
///
/// Requires `include=journals,attachments,relations` so the test fails if the
/// CLI stops asking for relations.
pub fn mock_issue_get_with_relations() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/issues/\d+\.json"))
        .and(query_param("include", "journals,attachments,relations"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issue": {
                "id": 123,
                "subject": "Test Issue",
                "project": {"id": 1, "name": "Test Project"},
                "status": {"id": 1, "name": "New"},
                "priority": {"id": 2, "name": "Normal"},
                "parent": {"id": 100},
                "relations": relations_fixture()
            }
        })))
}

/// Create a mock for an issues list; requires `include=relations`.
pub fn mock_issues_list_with_relations() -> Mock {
    Mock::given(method("GET"))
        .and(path("/issues.json"))
        .and(query_param("include", "relations"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "issues": [
                {
                    "id": 123,
                    "subject": "Test Issue",
                    "project": {"id": 1, "name": "Test Project"},
                    "status": {"id": 1, "name": "New"},
                    "priority": {"id": 2, "name": "Normal"},
                    "parent": {"id": 100},
                    "relations": relations_fixture()
                }
            ],
            "total_count": 1,
            "offset": 0,
            "limit": 25
        })))
}

/// Create a mock for listing an issue's relations.
pub fn mock_relations_list() -> Mock {
    Mock::given(method("GET"))
        .and(path_regex(r"/issues/\d+/relations\.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "relations": relations_fixture() })),
        )
}

/// Create a mock for creating a relation; requires the given request body.
pub fn mock_relation_create(body: serde_json::Value) -> Mock {
    Mock::given(method("POST"))
        .and(path("/issues/123/relations.json"))
        .and(body_partial_json(body))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "relation": {
                "id": 9, "issue_id": 123, "issue_to_id": 124,
                "relation_type": "precedes", "delay": 2
            }
        })))
}

/// Create a mock for deleting relation 9.
pub fn mock_relation_delete() -> Mock {
    Mock::given(method("DELETE"))
        .and(path("/relations/9.json"))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(204))
}

/// Create a mock for updating an issue; requires the given `parent_issue_id`.
pub fn mock_issue_update_parent(parent: serde_json::Value) -> Mock {
    mock_issue_update(
        serde_json::json!({"parent_issue_id": parent}),
        ResponseTemplate::new(204),
    )
}

/// Create a mock for creating an issue; requires `parent_issue_id`.
pub fn mock_issue_create_with_parent(parent: u32) -> Mock {
    Mock::given(method("POST"))
        .and(path("/issues.json"))
        .and(body_partial_json(
            serde_json::json!({"issue": {"parent_issue_id": parent}}),
        ))
        .and(header("X-Redmine-API-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "issue": {
                "id": 125,
                "subject": "Child",
                "project": {"id": 1, "name": "Test Project"},
                "status": {"id": 1, "name": "New"},
                "priority": {"id": 2, "name": "Normal"},
                "parent": {"id": parent}
            }
        })))
}

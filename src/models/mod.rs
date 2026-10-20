//! Data models for Redmine API responses.

pub mod attachment;
mod custom_field;
mod issue;
mod project;
mod query;
mod relation;
mod time_entry;
mod user;
mod version;

// Re-export for public API (may not be used internally but available for consumers)
#[allow(unused_imports)]
pub use attachment::{
    Attachment, AttachmentDownloaded, AttachmentList, AttachmentRef, AttachmentResponse,
    AttachmentUploaded, UploadResponse,
};
#[allow(unused_imports)]
pub use custom_field::{CustomField, CustomFieldValue};
pub use issue::{
    ClearableId, Issue, IssueList, IssueResponse, NewIssue, NewIssueRequest, UpdateIssue,
    UpdateIssueRequest,
};
// Re-export for internal use by client/endpoints.rs
pub(crate) use issue::SearchResults;
pub use project::{Project, ProjectList, ProjectResponse};
#[allow(unused_imports)]
pub use query::{Query, QueryList};
#[allow(unused_imports)]
pub use relation::{
    IssueRef, NewRelation, NewRelationRequest, Relation, RelationCreated, RelationDeleted,
    RelationList, RelationResponse, RelationsResponse,
};
pub use time_entry::{
    Activity, ActivityList, GroupByField, GroupedTimeEntries, NewTimeEntry, NewTimeEntryRequest,
    TimeEntry, TimeEntryCreated, TimeEntryDeleted, TimeEntryList, TimeEntryResponse,
    TimeEntryUpdated, UpdateTimeEntry, UpdateTimeEntryRequest,
};
pub use user::{CurrentUser, CurrentUserResponse};
// Re-export for public API
#[allow(unused_imports)]
pub use user::User;
#[allow(unused_imports)]
pub use version::{Version, VersionList, VersionRef};

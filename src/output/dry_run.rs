//! Dry-run preview: the write request that `--dry-run` would have sent.

use serde::Serialize;

use super::{MarkdownOutput, Meta};

/// A write request that was built but not sent because of `--dry-run`.
///
/// Returned as a successful result (exit code 0): in markdown it prints the
/// `DRY RUN:` header and body, in JSON it is the envelope's `data`.
#[derive(Debug, Clone, Serialize)]
pub struct DryRunPreview {
    /// Always `true`; lets JSON consumers tell a preview from a real result.
    pub dry_run: bool,
    pub method: String,
    pub path: String,
    /// Request body, omitted for requests without one (e.g. DELETE).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
}

impl DryRunPreview {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        body: Option<serde_json::Value>,
    ) -> Self {
        Self {
            dry_run: true,
            method: method.into(),
            path: path.into(),
            body,
        }
    }
}

impl MarkdownOutput for DryRunPreview {
    fn to_markdown(&self, _meta: &Meta) -> String {
        let mut output = format!("DRY RUN: {} {} (no request sent)\n", self.method, self.path);
        if let Some(body) = &self.body {
            let pretty = serde_json::to_string_pretty(body).unwrap_or_else(|_| body.to_string());
            output.push_str(&pretty);
            output.push('\n');
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_has_header_and_body() {
        let p = DryRunPreview::new(
            "PUT",
            "/issues/5.json",
            Some(serde_json::json!({"issue": {"notes": "x"}})),
        );
        let md = p.to_markdown(&Meta::default());
        assert!(md.starts_with("DRY RUN: PUT /issues/5.json (no request sent)\n"));
        assert!(md.contains("\"notes\": \"x\""));
    }

    #[test]
    fn json_omits_missing_body() {
        let p = DryRunPreview::new("DELETE", "/relations/3.json", None);
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["dry_run"], true);
        assert_eq!(v["method"], "DELETE");
        assert!(v.get("body").is_none());
    }
}

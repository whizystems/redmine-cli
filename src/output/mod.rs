//! Output formatting module - Markdown default, JSON envelope available.

mod dry_run;
mod envelope;
mod format;
pub mod markdown;

pub use dry_run::DryRunPreview;
pub use envelope::{Envelope, ErrorInfo, Meta};
pub use format::{Format, OutputFormat};
pub use markdown::MarkdownOutput;

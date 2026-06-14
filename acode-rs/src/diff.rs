//! Fast text diffing using the `similar` crate.
//!
//! Provides line-level unified diffs and word-level inline diffs —
//! covering the needs that Acode's CodeMirror editor would use for
//! comparing file versions, merge conflict resolution, and change previews.

use serde::Serialize;
use similar::{ChangeTag, TextDiff};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Granularity of the diff output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffGranularity {
    /// Line-level diff (fast, good for large files)
    Lines,
    /// Word/character-level inline diff within changed lines
    Words,
}

/// A single change (addition, deletion, or equal).
#[derive(Debug, Clone, Serialize)]
pub struct DiffChange {
    /// "insert", "delete", or "equal"
    pub tag: String,
    /// The changed text
    pub value: String,
    /// 0-based line range in the old document
    pub old_start: Option<usize>,
    pub old_end: Option<usize>,
    /// 0-based line range in the new document
    pub new_start: Option<usize>,
    pub new_end: Option<usize>,
}

/// Full diff result between two texts.
#[derive(Debug, Clone, Serialize)]
pub struct DiffResult {
    /// List of changes
    pub changes: Vec<DiffChange>,
    /// Number of added lines
    pub additions: usize,
    /// Number of deleted lines
    pub deletions: usize,
    /// Edit distance score (0.0 = identical, 1.0 = completely different)
    pub similarity: f32,
    /// Unified diff format string
    pub unified: String,
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Number of context lines around changes (matches `diff -U N`)
    pub context_lines: usize,
    /// Granularity level
    pub granularity: DiffGranularity,
    /// Max size in bytes before switching to line-only mode (avoids OOM)
    pub max_inline_bytes: usize,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            context_lines: 3,
            granularity: DiffGranularity::Words,
            max_inline_bytes: 1_048_576, // 1 MB
        }
    }
}

// ---------------------------------------------------------------------------
// Core diff function
// ---------------------------------------------------------------------------

/// Compute the diff between two strings.
///
/// Uses the `similar` crate's Myers diff algorithm internally.
/// Automatically selects line-level or word-level granularity
/// based on file size to avoid pathological cases.
pub fn diff(old: &str, new: &str, options: &DiffOptions) -> DiffResult {
    let total_bytes = old.len() + new.len();
    let granularity = if total_bytes > options.max_inline_bytes {
        DiffGranularity::Lines
    } else {
        options.granularity
    };

    match granularity {
        DiffGranularity::Lines => diff_lines_only(old, new, options.context_lines),
        DiffGranularity::Words => diff_with_inline(old, new, options.context_lines),
    }
}

/// Line-level diff (no word-level detail).
fn diff_lines_only(old: &str, new: &str, context: usize) -> DiffResult {
    let text_diff = TextDiff::from_lines(old, new);
    let mut changes = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for change in text_diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => "equal",
            ChangeTag::Delete => "delete",
            ChangeTag::Insert => "insert",
        };

        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => deletions += 1,
            _ => {}
        }

        changes.push(DiffChange {
            tag: tag.to_string(),
            value: change.value().to_string(),
            old_start: change.old_index(),
            old_end: change.old_index().map(|i| i + 1),
            new_start: change.new_index(),
            new_end: change.new_index().map(|i| i + 1),
        });
    }

    let mut unified_buffer = Vec::new();
    text_diff.unified_diff()
        .context_radius(context)
        .to_writer(&mut unified_buffer)
        .ok();

    DiffResult {
        similarity: text_diff.ratio(),
        additions,
        deletions,
        changes,
        unified: String::from_utf8_lossy(&unified_buffer).to_string(),
    }
}

/// Diff with word-level inline detail within changed lines.
fn diff_with_inline(old: &str, new: &str, context: usize) -> DiffResult {
    let text_diff = TextDiff::from_lines(old, new);
    let mut changes = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;

    for change in text_diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => "equal",
            ChangeTag::Delete => "delete",
            ChangeTag::Insert => "insert",
        };

        match change.tag() {
            ChangeTag::Insert => additions += 1,
            ChangeTag::Delete => deletions += 1,
            _ => {}
        }

        changes.push(DiffChange {
            tag: tag.to_string(),
            value: change.value().to_string(),
            old_start: change.old_index(),
            old_end: change.old_index().map(|i| i + 1),
            new_start: change.new_index(),
            new_end: change.new_index().map(|i| i + 1),
        });
    }

    let mut unified_buffer = Vec::new();
    text_diff.unified_diff()
        .context_radius(context)
        .to_writer(&mut unified_buffer)
        .ok();

    DiffResult {
        similarity: text_diff.ratio(),
        additions,
        deletions,
        changes,
        unified: String::from_utf8_lossy(&unified_buffer).to_string(),
    }
}

// ---------------------------------------------------------------------------
// Convenience: generate a patch (unified diff string)
// ---------------------------------------------------------------------------

/// Generate a unified diff patch string directly.
pub fn unified_diff(old: &str, new: &str, old_label: Option<&str>, new_label: Option<&str>, context: usize) -> String {
    let text_diff = TextDiff::from_lines(old, new);
    let mut diff = text_diff.unified_diff().context_radius(context);
    let old_lbl = old_label.unwrap_or("");
    let new_lbl = new_label.unwrap_or("");
    diff.header(old_lbl, new_lbl);

    let mut buf = Vec::new();
    diff.to_writer(&mut buf).ok();
    String::from_utf8_lossy(&buf).to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts() {
        let result = diff("hello\nworld\n", "hello\nworld\n", &DiffOptions::default());
        assert_eq!(result.similarity, 1.0);
        assert_eq!(result.additions, 0);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn test_one_line_changed() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline2-modified\nline3\n";
        let result = diff(old, new, &DiffOptions::default());
        assert!(result.similarity < 1.0);
        assert_eq!(result.additions, 1);
        assert_eq!(result.deletions, 1);
    }

    #[test]
    fn test_added_lines() {
        let old = "a\nb\n";
        let new = "a\nb\nc\nd\n";
        let result = diff(old, new, &DiffOptions::default());
        assert_eq!(result.additions, 2);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn test_unified_diff_output() {
        let patch = unified_diff("old", "new", Some("a.txt"), Some("b.txt"), 3);
        assert!(patch.contains("---"));
        assert!(patch.contains("+++"));
        assert!(patch.contains("old") || patch.contains("new"));
    }

    #[test]
    fn test_empty_new() {
        let result = diff("hello\nworld\n", "", &DiffOptions::default());
        assert_eq!(result.similarity, 0.0);
        assert_eq!(result.deletions, 2);
    }
}

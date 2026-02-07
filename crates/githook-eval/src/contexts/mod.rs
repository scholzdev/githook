//! Context types for the Git object model and built-in typed wrappers.
//!
//! Each context backs a specific object kind in the scripting language
//! (e.g. `git.branch`, `git.files`, files, strings, HTTP responses).

mod file;
mod git;
mod http;
mod primitives;

pub use file::{FileContext, PathContext};
pub use git::{AuthorInfo, BranchInfo, CommitInfo, DiffStats, GitContext, RemoteInfo};
pub use http::{HttpContext, HttpResponseContext};
pub use primitives::{ArrayContext, NumberContext, StringContext};

/// A collection of file paths grouped by Git status (`staged`, `modified`, etc.).
///
/// Backs the `git.files.*` object in the scripting language.
#[derive(Debug, Clone)]
pub struct FilesCollection {
    /// Files in the staging area.
    pub staged: Vec<String>,
    /// All tracked files.
    pub all: Vec<String>,
    /// Modified (tracked, changed) files.
    pub modified: Vec<String>,
    /// Newly added files.
    pub added: Vec<String>,
    /// Deleted files.
    pub deleted: Vec<String>,
    /// Unstaged (working-tree) changes.
    pub unstaged: Vec<String>,
}

/// Added and removed lines from the current diff.
///
/// Backs the `git.diff.*` object.
#[derive(Debug, Clone)]
pub struct DiffCollection {
    /// Lines added in the diff (prefixed with `+`).
    pub added_lines: Vec<String>,
    /// Lines removed in the diff (prefixed with `-`).
    pub removed_lines: Vec<String>,
}

/// Merge information context (`git.merge.*`).
#[derive(Debug, Clone)]
pub struct MergeContext {
    /// The source (incoming) branch.
    pub source: String,
    /// The target (current) branch.
    pub target: String,
}

impl FilesCollection {
    pub fn staged(&self) -> Vec<String> {
        self.staged.clone()
    }

    pub fn all(&self) -> Vec<String> {
        self.all.clone()
    }

    pub fn modified(&self) -> Vec<String> {
        self.modified.clone()
    }

    pub fn added(&self) -> Vec<String> {
        self.added.clone()
    }

    pub fn deleted(&self) -> Vec<String> {
        self.deleted.clone()
    }

    pub fn unstaged(&self) -> Vec<String> {
        self.unstaged.clone()
    }
}

impl DiffCollection {
    /// Returns a reference to the added lines.
    pub fn added_lines(&self) -> &[String] {
        &self.added_lines
    }

    /// Returns a reference to the removed lines.
    pub fn removed_lines(&self) -> &[String] {
        &self.removed_lines
    }
}

impl MergeContext {
    /// Returns the merge source branch name.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the merge target branch name.
    pub fn target(&self) -> &str {
        &self.target
    }
}

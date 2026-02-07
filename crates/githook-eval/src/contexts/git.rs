//! Git-related context types (branch, commit, author, remote, stats).

use super::{DiffCollection, FilesCollection};
use githook_macros::{callable_impl, docs};

/// Full Git context with branch, commit, author, remote, stats, files, and diff info.
#[derive(Debug, Clone)]
pub struct GitContext {
    pub branch: BranchInfo,
    pub commit: Option<CommitInfo>,
    pub author: AuthorInfo,
    pub remote: RemoteInfo,
    pub stats: DiffStats,
    pub files: FilesCollection,
    pub diff: DiffCollection,
    pub is_merge_commit: bool,
    pub has_conflicts: bool,
}

/// Branch name and helpers.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
}

/// Commit message and hash.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub message: String,
    pub hash: String,
}

/// Author name and email.
#[derive(Debug, Clone)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

/// Remote name and URL.
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

/// Diff statistics (files changed, additions, deletions).
#[derive(Debug, Default, Clone)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

impl Default for GitContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GitContext {
    /// Creates a new `GitContext` by querying the current repository state.
    pub fn new() -> Self {
        let git_stats = githook_git::get_diff_stats().unwrap_or_default();

        Self {
            branch: BranchInfo {
                name: githook_git::get_branch_name().unwrap_or_else(|_| "main".to_string()),
            },
            commit: None,
            author: AuthorInfo {
                name: githook_git::get_author_name().unwrap_or_default(),
                email: githook_git::get_author_email().unwrap_or_default(),
            },
            remote: RemoteInfo {
                name: "origin".to_string(),
                url: githook_git::get_remote_url().unwrap_or_default(),
            },
            stats: DiffStats {
                files_changed: git_stats.files_changed,
                additions: git_stats.additions,
                deletions: git_stats.deletions,
            },
            files: FilesCollection {
                staged: githook_git::get_staged_files("*").unwrap_or_default(),
                all: githook_git::get_all_files("*").unwrap_or_default(),
                modified: githook_git::get_modified_files("*").unwrap_or_default(),
                added: githook_git::get_added_files("*").unwrap_or_default(),
                deleted: githook_git::get_deleted_files("*").unwrap_or_default(),
                unstaged: githook_git::get_unstaged_files("*").unwrap_or_default(),
            },
            diff: DiffCollection {
                added_lines: githook_git::get_added_lines_array().unwrap_or_default(),
                removed_lines: githook_git::get_removed_lines_array().unwrap_or_default(),
            },
            is_merge_commit: githook_git::is_merge_commit().unwrap_or(false),
            has_conflicts: false,
        }
    }
}

#[callable_impl]
impl GitContext {
    #[docs(
        name = "git.is_merge_commit",
        description = "Checks if the current commit is a merge commit",
        example = "if git.is_merge_commit { print \"Merge commit\" }"
    )]
    #[property]
    pub fn is_merge_commit(&self) -> bool {
        self.is_merge_commit
    }

    #[docs(
        name = "git.has_conflicts",
        description = "Checks if there are merge conflicts",
        example = "if git.has_conflicts { warn \"Resolve conflicts first\" }"
    )]
    #[property]
    pub fn has_conflicts(&self) -> bool {
        self.has_conflicts
    }
}

#[callable_impl]
impl BranchInfo {
    #[docs(
        name = "git.branch.name",
        description = "Current Git branch name",
        example = "if git.branch.name == \"main\" { warn \"Direct push to main\" }"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[docs(
        name = "git.branch.is_main",
        description = "Checks if the current branch is main or master",
        example = "if git.branch.is_main { print \"On main branch\" }"
    )]
    #[property]
    pub fn is_main(&self) -> bool {
        matches!(self.name.as_str(), "main" | "master")
    }
}

#[callable_impl]
impl CommitInfo {
    #[docs(
        name = "git.commit.message",
        description = "Git commit message",
        example = "print git.commit.message"
    )]
    #[property]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    #[docs(
        name = "git.commit.hash",
        description = "Git commit hash",
        example = "print git.commit.hash"
    )]
    #[property]
    pub fn hash(&self) -> String {
        self.hash.clone()
    }
}

#[callable_impl]
impl AuthorInfo {
    #[docs(
        name = "git.author.name",
        description = "Git author name",
        example = "print git.author.name"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[docs(
        name = "git.author.email",
        description = "Git author email address",
        example = "print git.author.email"
    )]
    #[property]
    pub fn email(&self) -> String {
        self.email.clone()
    }
}

#[callable_impl]
impl RemoteInfo {
    #[docs(
        name = "git.remote.name",
        description = "Git remote name",
        example = "print git.remote.name"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[docs(
        name = "git.remote.url",
        description = "Git remote URL",
        example = "print git.remote.url"
    )]
    #[property]
    pub fn url(&self) -> String {
        self.url.clone()
    }
}

#[callable_impl]
impl DiffStats {
    #[docs(
        name = "git.stats.files_changed",
        description = "Number of files changed in the diff",
        example = "print git.stats.files_changed"
    )]
    #[property]
    pub fn files_changed(&self) -> f64 {
        self.files_changed as f64
    }

    #[docs(
        name = "git.stats.additions",
        description = "Number of additions in the diff",
        example = "print git.stats.additions"
    )]
    #[property]
    pub fn additions(&self) -> f64 {
        self.additions as f64
    }

    #[docs(
        name = "git.stats.deletions",
        description = "Number of deletions in the diff",
        example = "print git.stats.deletions"
    )]
    #[property]
    pub fn deletions(&self) -> f64 {
        self.deletions as f64
    }

    #[docs(
        name = "git.stats.modified_lines",
        description = "Total number of modified lines (additions + deletions)",
        example = "print git.stats.modified_lines"
    )]
    #[property]
    pub fn modified_lines(&self) -> f64 {
        (self.additions + self.deletions) as f64
    }
}

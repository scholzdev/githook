//! Builds the `git` top-level object tree.
//!
//! The `create_git_object` helper queries the repository state and
//! assembles the nested `Object` hierarchy that scripts see as
//! `git.branch`, `git.files.staged`, `git.diff`, etc.

use crate::contexts::GitContext;
use crate::value::{Object, Value};

use super::Executor;

impl Executor {
    /// Builds the top-level `git` value by querying the current repo.
    pub(super) fn create_git_object(&self) -> Value {
        let git_context = GitContext::new();

        let mut git = Object::new("Git").with_git_context(git_context.clone());

        let mut files_obj =
            Object::new("FilesCollection").with_files_context(git_context.files.clone());

        let all_files: Vec<Value> = git_context
            .files
            .all
            .iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("all", Value::Array(all_files));

        let staged_files: Vec<Value> = git_context
            .files
            .staged
            .iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("staged", Value::Array(staged_files));

        let modified_files: Vec<Value> = git_context
            .files
            .modified
            .iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("modified", Value::Array(modified_files));

        let added_files: Vec<Value> = git_context
            .files
            .added
            .iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("added", Value::Array(added_files));

        let deleted_files: Vec<Value> = git_context
            .files
            .deleted
            .iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("deleted", Value::Array(deleted_files));

        let unstaged_files: Vec<Value> = git_context
            .files
            .unstaged
            .iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("unstaged", Value::Array(unstaged_files));

        git.set("files", Value::Object(files_obj));

        let mut diff_obj = Object::new("DiffCollection");

        let added_lines: Vec<Value> = git_context
            .diff
            .added_lines
            .iter()
            .map(|line| Value::String(line.clone()))
            .collect();
        diff_obj.set("added_lines", Value::Array(added_lines));

        let removed_lines: Vec<Value> = git_context
            .diff
            .removed_lines
            .iter()
            .map(|line| Value::String(line.clone()))
            .collect();
        diff_obj.set("removed_lines", Value::Array(removed_lines));

        git.set("diff", Value::Object(diff_obj));

        let mut merge_obj = Object::new("MergeContext");

        let merge_source =
            githook_git::get_merge_source_branch().unwrap_or_else(|_| "unknown".to_string());
        let merge_target = githook_git::get_branch_name().unwrap_or_else(|_| "unknown".to_string());

        merge_obj.set("source", Value::String(merge_source));
        merge_obj.set("target", Value::String(merge_target));

        git.set("merge", Value::Object(merge_obj));

        let branch = Object::new("Branch").with_branch_context(git_context.branch.clone());
        git.set("branch", Value::Object(branch));

        let commit_value = if let Some(commit_info) = git_context.commit.clone() {
            let commit = Object::new("Commit").with_commit_context(commit_info);
            Value::Object(commit)
        } else {
            Value::Null
        };
        git.set("commit", commit_value);

        let author = Object::new("Author").with_author_context(git_context.author.clone());
        git.set("author", Value::Object(author));

        let remote = Object::new("Remote").with_remote_context(git_context.remote.clone());
        git.set("remote", Value::Object(remote));

        let stats = Object::new("Stats").with_stats_context(git_context.stats.clone());
        git.set("stats", Value::Object(stats));

        Value::Object(git)
    }
}

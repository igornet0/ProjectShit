//! History helpers for change tracking (`.brdd` snapshots, changelogs).

use git2::{DiffOptions, Oid};
use serde::{Deserialize, Serialize};

use crate::repository::{GitError, GitRepository};
use crate::status::GitCommitInfo;

/// Lines/files changed between two points in history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffStat {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
}

impl GitRepository {
    /// Full 40-char id of `HEAD`, `None` on an unborn branch.
    pub fn head_full(&self) -> Option<String> {
        self.repo()
            .head()
            .ok()
            .and_then(|h| h.target())
            .map(|oid| oid.to_string())
    }

    /// Branch name (`detached` when HEAD is not a branch).
    pub fn branch_name(&self) -> Option<String> {
        let head = self.repo().head().ok()?;
        Some(if head.is_branch() {
            head.shorthand().unwrap_or("HEAD").to_string()
        } else {
            "detached".to_string()
        })
    }

    /// Commits reachable from HEAD but not from `since` (newest first).
    /// An unknown `since` (rebased away, shallow clone) falls back to the
    /// latest `limit` commits.
    pub fn commits_since(&self, since: Option<&str>, limit: usize) -> Result<Vec<GitCommitInfo>, GitError> {
        let repo = self.repo();
        if repo.head().is_err() {
            return Ok(Vec::new());
        }
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        if let Some(oid) = since.and_then(|s| Oid::from_str(s).ok()) {
            if repo.find_commit(oid).is_ok() {
                revwalk.hide(oid)?;
            }
        }
        let mut out = Vec::new();
        for oid in revwalk.take(limit) {
            let commit = repo.find_commit(oid?)?;
            out.push(crate::status::commit_to_info(&commit));
        }
        Ok(out)
    }

    /// Tags whose target is the current HEAD commit.
    pub fn tags_at_head(&self) -> Vec<String> {
        let Some(head) = self.repo().head().ok().and_then(|h| h.peel_to_commit().ok()) else {
            return Vec::new();
        };
        let mut tags = Vec::new();
        let _ = self.repo().tag_foreach(|oid, name| {
            let points_at_head = self
                .repo()
                .find_object(oid, None)
                .ok()
                .and_then(|o| o.peel_to_commit().ok())
                .is_some_and(|c| c.id() == head.id());
            if points_at_head {
                if let Ok(name) = std::str::from_utf8(name) {
                    tags.push(name.trim_start_matches("refs/tags/").to_string());
                }
            }
            true
        });
        tags.sort();
        tags
    }

    /// Diff stat from `since` to HEAD (whole tree when `since` is unknown).
    pub fn diff_stat_since(&self, since: Option<&str>) -> Result<DiffStat, GitError> {
        let repo = self.repo();
        let Some(head_tree) = repo.head().ok().and_then(|h| h.peel_to_tree().ok()) else {
            return Ok(DiffStat::default());
        };
        let base_tree = since
            .and_then(|s| Oid::from_str(s).ok())
            .and_then(|oid| repo.find_commit(oid).ok())
            .and_then(|c| c.tree().ok());
        let mut opts = DiffOptions::new();
        let diff = repo.diff_tree_to_tree(base_tree.as_ref(), Some(&head_tree), Some(&mut opts))?;
        let stats = diff.stats()?;
        Ok(DiffStat {
            files_changed: stats.files_changed(),
            insertions: stats.insertions(),
            deletions: stats.deletions(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::path::Path;

    fn commit(repo: &Repository, path: &Path, file: &str, body: &str, msg: &str) -> Oid {
        std::fs::write(path.join(file), body).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("t", "t@example.com").unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap()
    }

    #[test]
    fn commits_since_and_diff_stat() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let first = commit(&repo, dir.path(), "a.txt", "one\n", "first");
        commit(&repo, dir.path(), "a.txt", "one\ntwo\n", "second");
        commit(&repo, dir.path(), "b.txt", "x\n", "third");
        repo.tag_lightweight("v1.0.0", &repo.head().unwrap().peel(git2::ObjectType::Commit).unwrap(), false)
            .unwrap();

        let g = GitRepository::open(dir.path()).unwrap();
        let since = g.commits_since(Some(&first.to_string()), 10).unwrap();
        assert_eq!(
            since.iter().map(|c| c.message.as_str()).collect::<Vec<_>>(),
            vec!["third", "second"]
        );
        let stat = g.diff_stat_since(Some(&first.to_string())).unwrap();
        assert_eq!(stat, DiffStat { files_changed: 2, insertions: 2, deletions: 0 });
        assert_eq!(g.tags_at_head(), vec!["v1.0.0"]);
        assert_eq!(g.head_full().unwrap().len(), 40);
        assert_eq!(g.commits_since(Some("not-a-sha"), 2).unwrap().len(), 2);
    }
}

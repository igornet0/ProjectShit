//! `.brdd` — a small, self-describing folder inside every project:
//!
//! | File | Content |
//! |------|---------|
//! | `README.md` | what the folder is |
//! | `project.json` | machine-readable analysis (stack, version, deps, structure, git, tasks) |
//! | `summary.md` | the same as a short human-readable brief |
//! | `versions.json` | version snapshots (version, commit, tags, size) |
//! | `CHANGELOG.md` | changes between snapshots (commits, diff stat) |
//! | `tasks.json` | the project's Project Hub tasks |
//! | `ai-summary.md` | free-form notes written by an AI agent / the user (never overwritten) |
//!
//! Everything except `ai-summary.md` is regenerated on refresh. In git repos
//! `.brdd/` is added to `.git/info/exclude` (local, never committed) unless
//! the caller opts out.

mod manifests;
mod render;
mod structure;

use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use project_hub_domain::{Project, Task, TaskStatus};
use project_hub_git::{DiffStat, GitCommitInfo, GitRepository};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use manifests::{infer_stack, Dependency, ManifestFacts};
pub use structure::{DirStat, LanguageStat, Structure};

pub const BRDD_DIR: &str = ".brdd";
pub const FORMAT_VERSION: u32 = 1;
const MAX_SNAPSHOTS: usize = 200;
const MAX_COMMITS_PER_SNAPSHOT: usize = 50;
const NOTES_FILE: &str = "ai-summary.md";
const MAX_NOTES_BYTES: usize = 256 * 1024;

#[derive(Debug, Error)]
pub enum BrddError {
    #[error("project folder not found: {0}")]
    MissingRoot(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFacts {
    pub id: String,
    pub name: String,
    pub language: String,
    pub project_type: String,
    pub root_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFacts {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub head_short: Option<String>,
    pub remote: Option<String>,
    pub dirty: bool,
    pub modified: usize,
    pub staged: usize,
    pub untracked: usize,
    pub ahead: usize,
    pub behind: usize,
    pub tags: Vec<String>,
    pub last_commit: Option<GitCommitInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmeFacts {
    pub file: String,
    pub title: Option<String>,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandFact {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCounts {
    pub total: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub done: usize,
    pub cancelled: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrddAnalysis {
    pub format: u32,
    pub analyzed_at: DateTime<Utc>,
    pub project: ProjectFacts,
    pub description: Option<String>,
    pub version: Option<String>,
    pub version_source: Option<String>,
    pub manifests: Vec<String>,
    pub stack: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub structure: Structure,
    pub commands: Vec<CommandFact>,
    pub git: Option<GitFacts>,
    pub readme: Option<ReadmeFacts>,
    pub license: Option<String>,
    pub tasks: TaskCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionSnapshot {
    /// 1-based sequence number.
    pub n: u32,
    pub at: DateTime<Utc>,
    pub version: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub commit_short: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub dirty: bool,
    /// `initial` | `version` | `commits` | `manual`
    pub reason: String,
    pub files: usize,
    pub loc: usize,
    /// Commits since the previous snapshot (newest first).
    #[serde(default)]
    pub commits: Vec<GitCommitInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffStat>,
}

#[derive(Debug, Clone, Copy)]
pub struct BrddOptions {
    /// Add `.brdd/` to `.git/info/exclude` so the folder never dirties the repo.
    pub exclude_from_git: bool,
    /// Record a snapshot even when version and commit did not change.
    pub force_snapshot: bool,
    /// Stop walking after this many files.
    pub max_files: usize,
}

impl Default for BrddOptions {
    fn default() -> Self {
        Self {
            exclude_from_git: true,
            force_snapshot: false,
            max_files: 20_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrddReport {
    pub dir: PathBuf,
    pub analysis: BrddAnalysis,
    pub snapshot: Option<VersionSnapshot>,
    pub snapshots: usize,
    pub files_written: Vec<String>,
}

/// Everything readable from an existing `.brdd` folder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrddBundle {
    pub exists: bool,
    pub dir: PathBuf,
    pub analysis: Option<BrddAnalysis>,
    pub versions: Vec<VersionSnapshot>,
    pub summary_md: Option<String>,
    pub changelog_md: Option<String>,
    pub notes_md: Option<String>,
}

pub fn brdd_dir(root: &Path) -> PathBuf {
    root.join(BRDD_DIR)
}

/// Analyse the project and (re)write its `.brdd` folder.
pub fn refresh(project: &Project, tasks: &[Task], opts: &BrddOptions) -> Result<BrddReport, BrddError> {
    let root = project.root_path.as_path();
    if !root.is_dir() {
        return Err(BrddError::MissingRoot(root.display().to_string()));
    }
    let analysis = analyze(project, tasks, opts.max_files);
    let dir = brdd_dir(root);
    std::fs::create_dir_all(&dir)?;
    if opts.exclude_from_git {
        // Best effort — a read-only .git must not block the analysis.
        if let Err(err) = exclude_from_git(root) {
            tracing::debug!(error = %err, path = %root.display(), "brdd.git_exclude_skipped");
        }
    }

    let mut versions = read_versions(&dir);
    let snapshot = next_snapshot(root, &analysis, versions.last(), opts.force_snapshot);
    if let Some(s) = &snapshot {
        versions.push(s.clone());
        if versions.len() > MAX_SNAPSHOTS {
            let drop = versions.len() - MAX_SNAPSHOTS;
            versions.drain(..drop);
        }
    }

    let mut written = Vec::new();
    let mut put = |name: &str, body: &[u8]| -> Result<(), BrddError> {
        write_atomic(&dir.join(name), body)?;
        written.push(name.to_string());
        Ok(())
    };
    put("README.md", render::readme().as_bytes())?;
    put("project.json", &serde_json::to_vec_pretty(&analysis)?)?;
    put("summary.md", render::summary(&analysis, &versions).as_bytes())?;
    put("versions.json", &serde_json::to_vec_pretty(&versions)?)?;
    put("CHANGELOG.md", render::changelog(&analysis.project.name, &versions).as_bytes())?;
    put("tasks.json", &serde_json::to_vec_pretty(&render::task_export(tasks))?)?;

    Ok(BrddReport {
        dir,
        snapshots: versions.len(),
        analysis,
        snapshot,
        files_written: written,
    })
}

/// Pure analysis (no writes).
pub fn analyze(project: &Project, tasks: &[Task], max_files: usize) -> BrddAnalysis {
    let root = project.root_path.as_path();
    let manifest = manifests::collect(root);
    let structure = structure::walk(root, max_files);
    let commands = project_hub_executor::providers::detect_commands(project)
        .into_iter()
        .map(|c| CommandFact {
            name: c.name,
            command: c.command,
        })
        .collect();

    BrddAnalysis {
        format: FORMAT_VERSION,
        analyzed_at: Utc::now(),
        project: ProjectFacts {
            id: project.id.to_string(),
            name: project.name.clone(),
            language: project.language.as_str().to_string(),
            project_type: project.project_type.as_str().to_string(),
            root_path: root.display().to_string(),
        },
        description: project.description.clone().or(manifest.description.clone()),
        version: manifest.version.clone(),
        version_source: manifest.version_source.clone(),
        stack: infer_stack(&manifest.dependencies),
        manifests: manifest.manifests,
        dependencies: manifest.dependencies,
        structure,
        commands,
        git: git_facts(root),
        readme: readme_facts(root),
        license: license(root),
        tasks: count_tasks(tasks),
    }
}

/// Read whatever exists in `<root>/.brdd`.
pub fn read(root: &Path) -> BrddBundle {
    let dir = brdd_dir(root);
    let text = |name: &str| std::fs::read_to_string(dir.join(name)).ok();
    BrddBundle {
        exists: dir.is_dir(),
        analysis: text("project.json").and_then(|t| serde_json::from_str(&t).ok()),
        versions: read_versions(&dir),
        summary_md: text("summary.md"),
        changelog_md: text("CHANGELOG.md"),
        notes_md: text(NOTES_FILE),
        dir,
    }
}

/// Write `ai-summary.md` (kept across refreshes).
pub fn write_notes(root: &Path, markdown: &str) -> Result<PathBuf, BrddError> {
    if !root.is_dir() {
        return Err(BrddError::MissingRoot(root.display().to_string()));
    }
    if markdown.len() > MAX_NOTES_BYTES {
        return Err(BrddError::Invalid(format!("notes exceed {MAX_NOTES_BYTES} bytes")));
    }
    let dir = brdd_dir(root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(NOTES_FILE);
    write_atomic(&path, markdown.as_bytes())?;
    Ok(path)
}

fn next_snapshot(
    root: &Path,
    analysis: &BrddAnalysis,
    last: Option<&VersionSnapshot>,
    force: bool,
) -> Option<VersionSnapshot> {
    let git = analysis.git.as_ref();
    let head = git.and_then(|g| g.head.clone());
    let reason = match last {
        None => "initial",
        Some(l) if l.version != analysis.version => "version",
        Some(l) if l.commit != head => "commits",
        Some(_) if force => "manual",
        Some(_) => return None,
    };

    let repo = git.and_then(|_| GitRepository::open(root).ok());
    let since = last.and_then(|l| l.commit.as_deref());
    let (commits, diff) = match &repo {
        Some(r) if last.is_some() => (
            r.commits_since(since, MAX_COMMITS_PER_SNAPSHOT).unwrap_or_default(),
            r.diff_stat_since(since).ok(),
        ),
        Some(r) => (r.commits_since(None, 10).unwrap_or_default(), None),
        None => (Vec::new(), None),
    };

    Some(VersionSnapshot {
        n: last.map(|l| l.n + 1).unwrap_or(1),
        at: analysis.analyzed_at,
        version: analysis.version.clone(),
        branch: git.and_then(|g| g.branch.clone()),
        commit: head,
        commit_short: git.and_then(|g| g.head_short.clone()),
        tags: git.map(|g| g.tags.clone()).unwrap_or_default(),
        dirty: git.is_some_and(|g| g.dirty),
        reason: reason.to_string(),
        files: analysis.structure.files,
        loc: analysis.structure.loc,
        commits,
        diff,
    })
}

fn read_versions(dir: &Path) -> Vec<VersionSnapshot> {
    std::fs::read_to_string(dir.join("versions.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn git_facts(root: &Path) -> Option<GitFacts> {
    let repo = GitRepository::open(root).ok()?;
    let status = repo.status().ok();
    let head = repo.head_full();
    Some(GitFacts {
        branch: repo.branch_name(),
        head_short: head.as_ref().map(|h| h[..7.min(h.len())].to_string()),
        head,
        remote: repo.origin_url(),
        dirty: status
            .as_ref()
            .is_some_and(|s| s.modified + s.staged + s.untracked > 0),
        modified: status.as_ref().map(|s| s.modified).unwrap_or(0),
        staged: status.as_ref().map(|s| s.staged).unwrap_or(0),
        untracked: status.as_ref().map(|s| s.untracked).unwrap_or(0),
        ahead: status.as_ref().map(|s| s.ahead).unwrap_or(0),
        behind: status.as_ref().map(|s| s.behind).unwrap_or(0),
        tags: repo.tags_at_head(),
        last_commit: repo.last_commit().ok().flatten(),
    })
}

fn readme_facts(root: &Path) -> Option<ReadmeFacts> {
    let entry = std::fs::read_dir(root).ok()?.filter_map(Result::ok).find(|e| {
        let name = e.file_name().to_string_lossy().to_ascii_lowercase();
        name == "readme.md" || name == "readme" || name == "readme.txt" || name == "readme.rst"
    })?;
    let text = std::fs::read_to_string(entry.path()).ok()?;
    let title = text
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches('#').trim().to_string());
    let excerpt = text
        .split("\n\n")
        .map(str::trim)
        .find(|p| {
            !p.is_empty()
                && !p.starts_with('#')
                && !p.starts_with('[')
                && !p.starts_with('!')
                && !p.starts_with('<')
                && !p.starts_with("```")
                && !p.starts_with('|')
        })
        .map(|p| truncate(&p.replace('\n', " "), 600));
    Some(ReadmeFacts {
        file: entry.file_name().to_string_lossy().to_string(),
        title,
        excerpt,
    })
}

fn license(root: &Path) -> Option<String> {
    let entry = std::fs::read_dir(root).ok()?.filter_map(Result::ok).find(|e| {
        e.file_name()
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("LICENSE")
    })?;
    let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
    let head = text.lines().take(3).collect::<Vec<_>>().join(" ").to_ascii_lowercase();
    Some(
        if head.contains("mit license") {
            "MIT"
        } else if head.contains("apache license") {
            "Apache-2.0"
        } else if head.contains("gnu general public license") {
            "GPL"
        } else if head.contains("bsd") {
            "BSD"
        } else {
            "custom"
        }
        .to_string(),
    )
}

fn count_tasks(tasks: &[Task]) -> TaskCounts {
    let mut c = TaskCounts {
        total: tasks.len(),
        ..Default::default()
    };
    for t in tasks {
        match t.status {
            TaskStatus::Todo => c.todo += 1,
            TaskStatus::InProgress => c.in_progress += 1,
            TaskStatus::Done => c.done += 1,
            TaskStatus::Cancelled => c.cancelled += 1,
        }
    }
    c
}

/// Append `.brdd/` to `.git/info/exclude` once.
fn exclude_from_git(root: &Path) -> std::io::Result<()> {
    let git_dir = root.join(".git");
    if !git_dir.is_dir() {
        return Ok(());
    }
    let info = git_dir.join("info");
    std::fs::create_dir_all(&info)?;
    let path = info.join("exclude");
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current.lines().any(|l| matches!(l.trim(), ".brdd" | ".brdd/" | "/.brdd" | "/.brdd/")) {
        return Ok(());
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    if !current.is_empty() && !current.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    file.write_all(b"# Project Hub analysis folder (local only)\n/.brdd/\n")?;
    Ok(())
}

fn write_atomic(path: &Path, body: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp-brdd");
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests;

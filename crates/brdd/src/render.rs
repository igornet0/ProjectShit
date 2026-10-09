//! Markdown / JSON renderers for the `.brdd` files.

use std::fmt::Write;

use project_hub_domain::Task;
use serde::Serialize;

use crate::{truncate, BrddAnalysis, VersionSnapshot};

pub fn readme() -> String {
    "# .brdd\n\n\
     Project facts maintained by **Project Hub** (and BoardDo agents).\n\n\
     | File | Content |\n\
     |------|---------|\n\
     | `summary.md` | short human-readable analysis |\n\
     | `project.json` | the same analysis, machine-readable |\n\
     | `versions.json` | version snapshots: version, commit, tags, size |\n\
     | `CHANGELOG.md` | changes between snapshots |\n\
     | `tasks.json` | Project Hub tasks of this project |\n\
     | `ai-summary.md` | notes written by an AI agent or by you — never overwritten |\n\n\
     All files except `ai-summary.md` are regenerated on refresh. In git repositories the\n\
     folder is listed in `.git/info/exclude` (local only); delete that line to commit it.\n"
        .to_string()
}

pub fn summary(a: &BrddAnalysis, versions: &[VersionSnapshot]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# {}", a.project.name);
    if let Some(desc) = a
        .description
        .as_deref()
        .or(a.readme.as_ref().and_then(|r| r.excerpt.as_deref()))
    {
        let _ = writeln!(out, "\n> {}", truncate(&desc.replace('\n', " "), 400));
    }

    let _ = writeln!(out, "\n| | |\n|---|---|");
    row(&mut out, "Version", a.version.as_deref().map(|v| match &a.version_source {
        Some(src) => format!("{v} ({src})"),
        None => v.to_string(),
    }));
    row(&mut out, "Language", Some(a.project.language.clone()));
    row(&mut out, "Type", Some(a.project.project_type.clone()));
    if !a.stack.is_empty() {
        row(&mut out, "Stack", Some(a.stack.join(", ")));
    }
    row(
        &mut out,
        "Size",
        Some(format!(
            "{} files, {} source files, {} LOC{}",
            a.structure.files,
            a.structure.source_files,
            a.structure.loc,
            if a.structure.truncated { " (partial)" } else { "" }
        )),
    );
    if !a.manifests.is_empty() {
        row(&mut out, "Manifests", Some(a.manifests.join(", ")));
    }
    row(&mut out, "License", a.license.clone());
    row(
        &mut out,
        "Quality signals",
        Some(
            [
                (a.structure.has_tests, "tests"),
                (a.structure.has_docker, "docker"),
                (!a.structure.ci.is_empty(), "CI"),
                (a.readme.is_some(), "README"),
            ]
            .iter()
            .map(|(on, label)| format!("{}{label}", if *on { "✓ " } else { "✗ " }))
            .collect::<Vec<_>>()
            .join(" · "),
        ),
    );
    row(&mut out, "Analyzed", Some(a.analyzed_at.format("%Y-%m-%d %H:%M UTC").to_string()));

    if !a.structure.languages.is_empty() {
        let _ = writeln!(out, "\n## Languages\n");
        for l in a.structure.languages.iter().take(8) {
            let _ = writeln!(out, "- {} — {} files, {} LOC", l.language, l.files, l.loc);
        }
    }
    if !a.structure.top_dirs.is_empty() {
        let _ = writeln!(out, "\n## Layout\n");
        for d in &a.structure.top_dirs {
            let _ = writeln!(out, "- `{}/` — {} files", d.name, d.files);
        }
    }

    let runtime: Vec<&str> = a
        .dependencies
        .iter()
        .filter(|d| d.kind == "runtime")
        .map(|d| d.name.as_str())
        .collect();
    if !a.dependencies.is_empty() {
        let _ = writeln!(
            out,
            "\n## Dependencies\n\n{} total ({} runtime). Main: {}",
            a.dependencies.len(),
            runtime.len(),
            runtime.iter().take(15).map(|d| format!("`{d}`")).collect::<Vec<_>>().join(", ")
        );
    }

    if !a.commands.is_empty() {
        let _ = writeln!(out, "\n## Commands\n");
        for c in a.commands.iter().take(15) {
            let _ = writeln!(out, "- **{}** — `{}`", c.name, c.command);
        }
    }

    if let Some(g) = &a.git {
        let _ = writeln!(out, "\n## Git\n");
        let _ = writeln!(
            out,
            "- Branch `{}` at `{}`{}",
            g.branch.as_deref().unwrap_or("?"),
            g.head_short.as_deref().unwrap_or("—"),
            if g.tags.is_empty() { String::new() } else { format!(" ({})", g.tags.join(", ")) }
        );
        if let Some(remote) = &g.remote {
            let _ = writeln!(out, "- Remote: {remote}");
        }
        let _ = writeln!(
            out,
            "- Working tree: {}",
            if g.dirty {
                format!("{} modified, {} staged, {} untracked", g.modified, g.staged, g.untracked)
            } else {
                "clean".into()
            }
        );
        if g.ahead + g.behind > 0 {
            let _ = writeln!(out, "- Upstream: {} ahead, {} behind", g.ahead, g.behind);
        }
        if let Some(c) = &g.last_commit {
            let _ = writeln!(
                out,
                "- Last commit: {} — {} ({})",
                c.hash,
                c.message,
                c.date.format("%Y-%m-%d")
            );
        }
    }

    let t = &a.tasks;
    if t.total > 0 {
        let _ = writeln!(
            out,
            "\n## Tasks\n\n{} total — {} todo, {} in progress, {} done, {} cancelled",
            t.total, t.todo, t.in_progress, t.done, t.cancelled
        );
    }

    if let Some(last) = versions.last() {
        let _ = writeln!(
            out,
            "\n## Versions\n\n{} snapshot(s). Latest #{} · {} · {}",
            versions.len(),
            last.n,
            last.version.as_deref().unwrap_or("no version"),
            last.at.format("%Y-%m-%d")
        );
        let _ = writeln!(out, "See `CHANGELOG.md`.");
    }
    out
}

fn row(out: &mut String, key: &str, value: Option<String>) {
    if let Some(v) = value.filter(|v| !v.is_empty()) {
        let _ = writeln!(out, "| {key} | {} |", v.replace('|', "\\|"));
    }
}

pub fn changelog(name: &str, versions: &[VersionSnapshot]) -> String {
    let mut out = format!(
        "# Changelog — {name}\n\n_Generated by Project Hub from `versions.json`; rewritten on every refresh._\n"
    );
    for s in versions.iter().rev() {
        let _ = writeln!(
            out,
            "\n## #{} · {} · {}{}{}",
            s.n,
            s.version.as_deref().unwrap_or("no version"),
            s.at.format("%Y-%m-%d %H:%M"),
            s.commit_short
                .as_deref()
                .map(|c| format!(" · `{c}`"))
                .unwrap_or_default(),
            s.branch.as_deref().map(|b| format!(" ({b})")).unwrap_or_default()
        );
        let reason = match s.reason.as_str() {
            "initial" => "first snapshot",
            "version" => "version changed",
            "commits" => "new commits",
            "manual" => "manual snapshot",
            other => other,
        };
        let mut meta = vec![reason.to_string(), format!("{} files, {} LOC", s.files, s.loc)];
        if let Some(d) = s.diff {
            meta.push(format!(
                "{} files changed, +{} / −{}",
                d.files_changed, d.insertions, d.deletions
            ));
        }
        if !s.tags.is_empty() {
            meta.push(format!("tags: {}", s.tags.join(", ")));
        }
        if s.dirty {
            meta.push("uncommitted changes".into());
        }
        let _ = writeln!(out, "\n_{}_\n", meta.join(" · "));
        for c in &s.commits {
            let _ = writeln!(
                out,
                "- `{}` {} — {}, {}",
                c.hash,
                c.message,
                c.author,
                c.date.format("%Y-%m-%d")
            );
        }
    }
    out
}

#[derive(Serialize)]
pub struct TaskExport<'a> {
    pub id: String,
    pub title: &'a str,
    pub status: &'static str,
    pub priority: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_at: Option<String>,
    pub updated_at: String,
}

pub fn task_export(tasks: &[Task]) -> Vec<TaskExport<'_>> {
    tasks
        .iter()
        .map(|t| TaskExport {
            id: t.id.to_string(),
            title: &t.title,
            status: t.status.as_str(),
            priority: t.priority.as_str(),
            description: t.description.as_deref(),
            due_at: t.due_at.map(|d| d.to_rfc3339()),
            updated_at: t.updated_at.to_rfc3339(),
        })
        .collect()
}

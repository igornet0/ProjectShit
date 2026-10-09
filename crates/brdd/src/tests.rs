use std::path::Path;

use chrono::Utc;
use git2::{Repository, Signature};
use project_hub_domain::{
    Language, Project, ProjectId, ProjectStatus, ProjectType, Task, TaskId, TaskPriority, TaskStatus,
};

use super::*;

fn project(root: &Path) -> Project {
    let now = Utc::now();
    Project {
        id: ProjectId::new(),
        name: "demo".into(),
        root_path: root.to_path_buf(),
        language: Language::Rust,
        project_type: ProjectType::Application,
        status: ProjectStatus::Active,
        description: None,
        icon: None,
        group_name: None,
        github_repo_id: None,
        remote_url: None,
        last_opened_at: None,
        last_modified_at: None,
        created_at: now,
        updated_at: now,
    }
}

fn task(p: &Project, title: &str, status: TaskStatus) -> Task {
    let now = Utc::now();
    Task {
        id: TaskId::new(),
        project_id: Some(p.id),
        title: title.into(),
        description: None,
        status,
        priority: TaskPriority::High,
        due_at: None,
        recurrence: None,
        series_id: None,
        created_at: now,
        updated_at: now,
    }
}

fn commit_all(repo: &Repository, msg: &str) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = Signature::now("dev", "dev@example.com").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap();
}

fn cargo_toml(version: &str) -> String {
    format!("[package]\nname = \"demo\"\nversion = \"{version}\"\ndescription = \"Demo app\"\n\n[dependencies]\ntokio = \"1\"\naxum = \"0.8\"\n")
}

#[test]
fn refresh_writes_folder_and_tracks_versions() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let repo = Repository::init(root).unwrap();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), cargo_toml("0.1.0")).unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
    std::fs::write(root.join("README.md"), "# Demo\n\nA tiny demo service.\n").unwrap();
    commit_all(&repo, "init");

    let p = project(root);
    let tasks = vec![task(&p, "Ship it", TaskStatus::Todo), task(&p, "Write docs", TaskStatus::Done)];
    let opts = BrddOptions::default();

    // 1. Initial snapshot.
    let r1 = refresh(&p, &tasks, &opts).unwrap();
    assert_eq!(r1.snapshot.as_ref().unwrap().reason, "initial");
    assert_eq!(r1.analysis.version.as_deref(), Some("0.1.0"));
    assert_eq!(r1.analysis.stack, vec!["Tokio", "Axum"]);
    assert_eq!(r1.analysis.tasks.total, 2);
    assert_eq!(r1.analysis.readme.as_ref().unwrap().excerpt.as_deref(), Some("A tiny demo service."));
    for f in ["README.md", "project.json", "summary.md", "versions.json", "CHANGELOG.md", "tasks.json"] {
        assert!(root.join(".brdd").join(f).is_file(), "{f}");
    }
    let exclude = std::fs::read_to_string(root.join(".git/info/exclude")).unwrap();
    assert!(exclude.contains("/.brdd/"));
    // .brdd never dirties the repo.
    assert!(!r1.analysis.git.as_ref().unwrap().dirty);
    let r_again = refresh(&p, &tasks, &opts).unwrap();
    assert!(r_again.analysis.git.as_ref().unwrap().untracked == 0);
    assert!(r_again.snapshot.is_none(), "nothing changed → no snapshot");
    assert_eq!(
        std::fs::read_to_string(root.join(".git/info/exclude")).unwrap().matches("/.brdd/").count(),
        1
    );

    // 2. New commit → "commits" snapshot listing it.
    std::fs::write(root.join("src/lib.rs"), "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();
    commit_all(&repo, "feat: add lib");
    let r2 = refresh(&p, &tasks, &opts).unwrap();
    let s2 = r2.snapshot.unwrap();
    assert_eq!(s2.reason, "commits");
    assert_eq!(s2.commits.len(), 1);
    assert_eq!(s2.commits[0].message, "feat: add lib");
    assert_eq!(s2.diff.unwrap().files_changed, 1);

    // 3. Version bump → "version" snapshot.
    std::fs::write(root.join("Cargo.toml"), cargo_toml("0.2.0")).unwrap();
    commit_all(&repo, "release 0.2.0");
    let r3 = refresh(&p, &tasks, &opts).unwrap();
    assert_eq!(r3.snapshot.unwrap().reason, "version");
    assert_eq!(r3.snapshots, 3);

    let changelog = std::fs::read_to_string(root.join(".brdd/CHANGELOG.md")).unwrap();
    assert!(changelog.find("#3 · 0.2.0").unwrap() < changelog.find("#1 · 0.1.0").unwrap());
    assert!(changelog.contains("feat: add lib"));

    // Notes survive refreshes; bundle reads everything back.
    write_notes(root, "## AI\nLooks healthy.").unwrap();
    refresh(&p, &tasks, &BrddOptions { force_snapshot: true, ..opts }).unwrap();
    let bundle = read(root);
    assert!(bundle.exists);
    assert_eq!(bundle.notes_md.as_deref(), Some("## AI\nLooks healthy."));
    assert_eq!(bundle.versions.len(), 4);
    assert_eq!(bundle.versions.last().unwrap().reason, "manual");
    assert!(bundle.summary_md.unwrap().contains("| Version | 0.2.0 (Cargo.toml) |"));
    let tasks_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".brdd/tasks.json")).unwrap()).unwrap();
    assert_eq!(tasks_json.as_array().unwrap().len(), 2);
}

#[test]
fn works_without_git_and_respects_opt_out() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("package.json"), r#"{"name":"web","version":"1.4.0","dependencies":{"react":"^19"},"devDependencies":{"vite":"^8"}}"#).unwrap();
    std::fs::write(root.join("index.ts"), "export const x = 1\n").unwrap();
    let p = project(root);
    let r = refresh(&p, &[], &BrddOptions { exclude_from_git: false, ..Default::default() }).unwrap();
    assert!(r.analysis.git.is_none());
    assert_eq!(r.analysis.version.as_deref(), Some("1.4.0"));
    assert_eq!(r.analysis.stack, vec!["React", "Vite"]);
    let s = r.snapshot.unwrap();
    assert_eq!(s.reason, "initial");
    assert!(s.commit.is_none());
    assert!(refresh(&p, &[], &BrddOptions::default()).unwrap().snapshot.is_none());
}

#[test]
fn missing_root_is_an_error() {
    let mut p = project(Path::new("/definitely/not/here"));
    p.name = "ghost".into();
    assert!(matches!(refresh(&p, &[], &BrddOptions::default()), Err(BrddError::MissingRoot(_))));
    assert!(!read(Path::new("/definitely/not/here")).exists);
}

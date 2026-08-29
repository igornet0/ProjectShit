use std::path::Path;

use project_hub_domain::{Language, ProjectInfo, ProjectType};

use crate::detector::ProjectDetector;

pub struct GoDetector;

impl ProjectDetector for GoDetector {
    fn detect(&self, path: &Path) -> Option<ProjectInfo> {
        let go_mod = path.join("go.mod");
        if !go_mod.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&go_mod).ok()?;
        let name = content
            .lines()
            .find(|l| l.starts_with("module "))
            .map(|l| l.trim_start_matches("module ").trim().to_string())
            .unwrap_or_else(|| path_name(path));

        Some(ProjectInfo {
            name,
            root_path: path.to_path_buf(),
            language: Language::Go,
            project_type: ProjectType::Application,
            has_git: path.join(".git").exists(),
            packages: Vec::new(),
        })
    }
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

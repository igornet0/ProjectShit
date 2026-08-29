use std::path::Path;

use project_hub_domain::{Language, ProjectInfo, ProjectType};

use crate::detector::ProjectDetector;

pub struct PythonDetector;

impl ProjectDetector for PythonDetector {
    fn detect(&self, path: &Path) -> Option<ProjectInfo> {
        let markers = ["pyproject.toml", "requirements.txt", "setup.py", "setup.cfg"];
        let has_marker = markers.iter().any(|m| path.join(m).exists());
        if !has_marker {
            return None;
        }

        let name = if path.join("pyproject.toml").exists() {
            read_pyproject_name(path).unwrap_or_else(|| path_name(path))
        } else {
            path_name(path)
        };

        Some(ProjectInfo {
            name,
            root_path: path.to_path_buf(),
            language: Language::Python,
            project_type: ProjectType::Application,
            has_git: path.join(".git").exists(),
            packages: Vec::new(),
        })
    }
}

fn read_pyproject_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path.join("pyproject.toml")).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;
    parsed
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

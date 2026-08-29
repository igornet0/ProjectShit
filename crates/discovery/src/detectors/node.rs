use std::path::Path;

use project_hub_domain::{Language, ProjectInfo, ProjectType};

use crate::detector::ProjectDetector;

pub struct NodeDetector;

impl ProjectDetector for NodeDetector {
    fn detect(&self, path: &Path) -> Option<ProjectInfo> {
        let package_json = path.join("package.json");
        if !package_json.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&package_json).ok()?;
        let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

        let name = parsed
            .get("name")
            .and_then(|n| n.as_str())
            .map(String::from)
            .unwrap_or_else(|| path_name(path));

        let project_type = if parsed
            .get("dependencies")
            .and_then(|d| d.get("react"))
            .is_some()
            || parsed
                .get("dependencies")
                .and_then(|d| d.get("next"))
                .is_some()
            || parsed
                .get("dependencies")
                .and_then(|d| d.get("vue"))
                .is_some()
        {
            ProjectType::Web
        } else {
            ProjectType::Application
        };

        let language = if parsed
            .get("devDependencies")
            .and_then(|d| d.get("typescript"))
            .is_some()
            || path.join("tsconfig.json").exists()
        {
            Language::TypeScript
        } else {
            Language::JavaScript
        };

        Some(ProjectInfo {
            name,
            root_path: path.to_path_buf(),
            language,
            project_type,
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

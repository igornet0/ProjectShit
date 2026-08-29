use std::path::Path;

use project_hub_domain::{Language, ProjectInfo, ProjectType};

use crate::detector::ProjectDetector;

pub struct JavaDetector;

impl ProjectDetector for JavaDetector {
    fn detect(&self, path: &Path) -> Option<ProjectInfo> {
        let has_pom = path.join("pom.xml").exists();
        let has_gradle = path.join("build.gradle").exists() || path.join("build.gradle.kts").exists();

        if !has_pom && !has_gradle {
            return None;
        }

        Some(ProjectInfo {
            name: path_name(path),
            root_path: path.to_path_buf(),
            language: Language::Java,
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

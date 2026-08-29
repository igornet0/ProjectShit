use std::path::Path;

use project_hub_domain::{Language, ProjectInfo, ProjectType};

use crate::detector::ProjectDetector;

/// Fallback detector for Makefile-based or git-only directories.
pub struct GenericDetector;

impl ProjectDetector for GenericDetector {
    fn detect(&self, path: &Path) -> Option<ProjectInfo> {
        let has_makefile = path.join("Makefile").exists();
        let has_git = path.join(".git").exists();

        if !has_makefile && !has_git {
            return None;
        }

        Some(ProjectInfo {
            name: path_name(path),
            root_path: path.to_path_buf(),
            language: Language::Unknown,
            project_type: if has_makefile {
                ProjectType::Application
            } else {
                ProjectType::Unknown
            },
            has_git,
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

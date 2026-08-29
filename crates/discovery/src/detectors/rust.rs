use std::path::Path;

use project_hub_domain::{Language, ProjectInfo, ProjectType};

use crate::detector::ProjectDetector;

pub struct RustDetector;

impl ProjectDetector for RustDetector {
    fn detect(&self, path: &Path) -> Option<ProjectInfo> {
        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&cargo_toml).ok()?;
        let parsed: toml::Value = toml::from_str(&content).ok()?;

        let name = parsed
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .map(String::from)
            .unwrap_or_else(|| path_name(path));

        let project_type = if parsed.get("workspace").is_some() {
            ProjectType::Workspace
        } else if parsed
            .get("package")
            .and_then(|p| p.get("description"))
            .and_then(|d| d.as_str())
            .is_some_and(|d| d.contains("library"))
        {
            ProjectType::Library
        } else {
            ProjectType::Application
        };

        let packages = extract_workspace_members(&parsed, path);

        Some(ProjectInfo {
            name,
            root_path: path.to_path_buf(),
            language: Language::Rust,
            project_type,
            has_git: path.join(".git").exists(),
            packages,
        })
    }
}

fn extract_workspace_members(parsed: &toml::Value, root: &Path) -> Vec<String> {
    let Some(members) = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    else {
        return Vec::new();
    };

    members
        .iter()
        .filter_map(|m| m.as_str())
        .flat_map(|pattern| expand_glob_pattern(root, pattern))
        .collect()
}

fn expand_glob_pattern(root: &Path, pattern: &str) -> Vec<String> {
    if pattern.contains('*') {
        return Vec::new();
    }
    let member_path = root.join(pattern);
    if member_path.join("Cargo.toml").exists() {
        vec![pattern.to_string()]
    } else {
        Vec::new()
    }
}

fn path_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detects_rust_binary_project() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "my-app"
version = "0.1.0"
"#,
        )
        .unwrap();

        let detector = RustDetector;
        let info = detector.detect(dir.path()).unwrap();
        assert_eq!(info.name, "my-app");
        assert_eq!(info.language, Language::Rust);
        assert_eq!(info.project_type, ProjectType::Application);
    }

    #[test]
    fn detects_rust_workspace() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crate-a", "crate-b"]
"#,
        )
        .unwrap();

        let detector = RustDetector;
        let info = detector.detect(dir.path()).unwrap();
        assert_eq!(info.project_type, ProjectType::Workspace);
    }
}

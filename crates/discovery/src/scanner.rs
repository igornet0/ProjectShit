use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use project_hub_domain::ProjectInfo;
use tracing::{debug, info};
use walkdir::WalkDir;

use crate::detector::ProjectDetector;
use crate::detectors::all_detectors;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".cargo",
    ".idea",
    ".vscode",
];

pub struct ProjectScanner {
    detectors: Vec<Arc<dyn ProjectDetector>>,
    max_depth: usize,
}

impl Default for ProjectScanner {
    fn default() -> Self {
        Self {
            detectors: all_detectors(),
            max_depth: 4,
        }
    }
}

impl ProjectScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scan_directory(&self, root: &Path) -> Vec<ProjectInfo> {
        let mut found = Vec::new();
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();

        info!(path = %root.display(), "starting project scan");

        // Prune skipped directories up front so WalkDir never descends into
        // node_modules/target/etc.
        for entry in WalkDir::new(root)
            .follow_links(false)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_entry(|e| e.depth() == 0 || !is_skipped_dir(e))
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_dir() {
                continue;
            }
            let path = entry.path();

            if let Some(info) = self.detect_at(path) {
                let canonical = info.root_path.clone();
                if seen_paths.insert(canonical) {
                    debug!(name = %info.name, path = %info.root_path.display(), "project detected");
                    found.push(info);
                }
            }
        }

        found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        info!(count = found.len(), "scan complete");
        found
    }

    /// Detect a single project at the given path (no recursive scan).
    pub fn detect_at(&self, path: &Path) -> Option<ProjectInfo> {
        for detector in &self.detectors {
            if let Some(info) = detector.detect(path) {
                return Some(info);
            }
        }
        None
    }
}

fn is_skipped_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|s| SKIP_DIRS.contains(&s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scans_multiple_projects() {
        let root = tempdir().unwrap();

        let rust_proj = root.path().join("avrora");
        std::fs::create_dir_all(&rust_proj).unwrap();
        std::fs::write(
            rust_proj.join("Cargo.toml"),
            "[package]\nname = \"avrora\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let py_proj = root.path().join("minesweeper");
        std::fs::create_dir_all(&py_proj).unwrap();
        std::fs::write(py_proj.join("requirements.txt"), "numpy\n").unwrap();

        let scanner = ProjectScanner::new();
        let projects = scanner.scan_directory(root.path());

        assert_eq!(projects.len(), 2);
    }
}

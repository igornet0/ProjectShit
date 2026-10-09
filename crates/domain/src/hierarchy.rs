use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{ProjectId, ProjectListItem};

/// Best-effort canonical path for stable prefix comparisons.
pub fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// True when `child` is a strict subdirectory of `parent`.
pub fn is_strict_child(parent: &Path, child: &Path) -> bool {
    if parent == child {
        return false;
    }

    let parent_components: Vec<_> = parent.components().collect();
    let child_components: Vec<_> = child.components().collect();

    if child_components.len() <= parent_components.len() {
        return false;
    }

    child_components[..parent_components.len()] == parent_components[..]
}

/// Derive parent/child relationships from project root paths.
pub fn enrich_with_hierarchy(items: &mut [ProjectListItem]) {
    let n = items.len();
    if n == 0 {
        return;
    }

    let ids: Vec<ProjectId> = items.iter().map(|item| item.project.id).collect();
    // Root paths are normalized when stored, so no canonicalize() here.
    let index_by_path: HashMap<&Path, usize> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.project.root_path.as_path(), i))
        .collect();

    // Nearest registered ancestor is the immediate parent: O(n * depth)
    // instead of comparing every pair of paths.
    let parent_idx: Vec<Option<usize>> = items
        .iter()
        .map(|item| {
            item.project
                .root_path
                .ancestors()
                .skip(1)
                .find_map(|ancestor| index_by_path.get(ancestor).copied())
        })
        .collect();

    let mut child_counts = vec![0u32; n];
    for parent in &parent_idx {
        if let Some(j) = parent {
            child_counts[*j] += 1;
        }
    }

    for (i, item) in items.iter_mut().enumerate() {
        let depth = {
            let mut depth = 0u32;
            let mut current = parent_idx[i];
            while let Some(j) = current {
                depth += 1;
                current = parent_idx[j];
            }
            depth
        };

        item.parent_id = parent_idx[i].map(|j| ids[j]);
        item.child_count = child_counts[i];
        item.depth = depth;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::Utc;

    use super::*;
    use crate::{
        Language, Project, ProjectId, ProjectListItem, ProjectStatus, ProjectType,
    };

    fn sample_project(name: &str, path: &str) -> ProjectListItem {
        let now = Utc::now();
        ProjectListItem {
            project: Project {
                id: ProjectId::new(),
                name: name.into(),
                root_path: PathBuf::from(path),
                language: Language::Unknown,
                project_type: ProjectType::Unknown,
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
            },
            has_git: false,
            git_dirty: false,
            parent_id: None,
            child_count: 0,
            depth: 0,
            folder_id: None,
        }
    }

    #[test]
    fn detects_nested_projects() {
        let mut items = vec![
            sample_project("root", "/tmp/workspace/app"),
            sample_project("frontend", "/tmp/workspace/app/frontend"),
            sample_project("other", "/tmp/other"),
        ];

        enrich_with_hierarchy(&mut items);

        assert_eq!(items[0].child_count, 1);
        assert_eq!(items[0].depth, 0);
        assert_eq!(items[1].parent_id, Some(items[0].project.id));
        assert_eq!(items[1].depth, 1);
        assert_eq!(items[2].parent_id, None);
    }

    #[test]
    fn sibling_paths_are_not_nested() {
        assert!(!is_strict_child(
            Path::new("/tmp/foo-bar"),
            Path::new("/tmp/foo")
        ));
    }
}

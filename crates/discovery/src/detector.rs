use std::path::Path;

use project_hub_domain::ProjectInfo;

pub trait ProjectDetector: Send + Sync {
    fn detect(&self, path: &Path) -> Option<ProjectInfo>;
}

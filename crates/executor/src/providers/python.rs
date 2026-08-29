use project_hub_domain::{Language, Project, ProjectCommand};

use super::{make_command, CommandProvider};

pub struct PythonCommandProvider;

impl CommandProvider for PythonCommandProvider {
    fn detect(&self, project: &Project) -> Vec<ProjectCommand> {
        if project.language != Language::Python {
            return Vec::new();
        }

        vec![
            make_command(project, "Test", "pytest"),
            make_command(project, "Run", "python -m main"),
        ]
    }
}

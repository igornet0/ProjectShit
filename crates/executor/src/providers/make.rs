use project_hub_domain::{Project, ProjectCommand};

use super::{make_command, CommandProvider};

pub struct MakeCommandProvider;

impl CommandProvider for MakeCommandProvider {
    fn detect(&self, project: &Project) -> Vec<ProjectCommand> {
        if !project.root_path.join("Makefile").exists() {
            return Vec::new();
        }

        vec![
            make_command(project, "Make", "make"),
            make_command(project, "Make Test", "make test"),
        ]
    }
}

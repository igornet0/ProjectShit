use project_hub_domain::{Language, Project, ProjectCommand};

use super::{make_command, CommandProvider};

pub struct NpmCommandProvider;

impl CommandProvider for NpmCommandProvider {
    fn detect(&self, project: &Project) -> Vec<ProjectCommand> {
        if !matches!(project.language, Language::JavaScript | Language::TypeScript) {
            return Vec::new();
        }

        vec![
            make_command(project, "Dev", "npm run dev"),
            make_command(project, "Test", "npm test"),
            make_command(project, "Build", "npm run build"),
        ]
    }
}

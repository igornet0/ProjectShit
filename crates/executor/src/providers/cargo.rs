use project_hub_domain::{Language, Project, ProjectCommand};

use super::{make_command, CommandProvider};

pub struct CargoCommandProvider;

impl CommandProvider for CargoCommandProvider {
    fn detect(&self, project: &Project) -> Vec<ProjectCommand> {
        if project.language != Language::Rust {
            return Vec::new();
        }

        vec![
            make_command(project, "Build", "cargo build"),
            make_command(project, "Test", "cargo test"),
            make_command(project, "Run", "cargo run"),
            make_command(project, "Clippy", "cargo clippy"),
        ]
    }
}

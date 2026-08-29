use chrono::Utc;
use project_hub_domain::{Project, ProjectCommand, ProjectCommandId};

pub trait CommandProvider: Send + Sync {
    fn detect(&self, project: &Project) -> Vec<ProjectCommand>;
}

pub mod cargo;
pub mod make;
pub mod npm;
pub mod python;

pub use cargo::CargoCommandProvider;
pub use make::MakeCommandProvider;
pub use npm::NpmCommandProvider;
pub use python::PythonCommandProvider;

use std::sync::Arc;

pub fn all_providers() -> Vec<Arc<dyn CommandProvider>> {
    vec![
        Arc::new(CargoCommandProvider),
        Arc::new(NpmCommandProvider),
        Arc::new(PythonCommandProvider),
        Arc::new(MakeCommandProvider),
    ]
}

pub fn detect_commands(project: &Project) -> Vec<ProjectCommand> {
    let mut commands = Vec::new();
    for provider in all_providers() {
        commands.extend(provider.detect(project));
    }
    commands
}

fn make_command(project: &Project, name: &str, command: &str) -> ProjectCommand {
    ProjectCommand {
        id: ProjectCommandId::new(),
        project_id: project.id,
        name: name.to_string(),
        command: command.to_string(),
        created_at: Utc::now(),
    }
}

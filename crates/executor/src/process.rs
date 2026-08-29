use std::path::Path;
use std::process::Stdio;

use project_hub_domain::Project;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("failed to spawn process: {0}")]
    Spawn(String),

    #[error("process failed with exit code {0}")]
    NonZeroExit(i32),
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub async fn run_in_directory(
    cwd: &Path,
    command: &str,
) -> Result<ProcessOutput, ProcessError> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| ProcessError::Spawn(e.to_string()))?;

    Ok(ProcessOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

pub struct CommandService;

impl CommandService {
    pub async fn run(project: &Project, command: &str) -> Result<ProcessOutput, ProcessError> {
        tracing::info!(
            project = %project.name,
            command = command,
            cwd = %project.root_path.display(),
            "running command"
        );
        run_in_directory(&project.root_path, command).await
    }
}

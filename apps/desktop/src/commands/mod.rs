use std::sync::Arc;

use crate::services::AppState;

mod brdd;
mod editors;
mod folders;
mod git;
mod github_cmd;
mod projects;
mod tasks;

pub use brdd::*;
pub use editors::*;
pub use folders::*;
pub use git::*;
pub use github_cmd::*;
pub use projects::*;
pub use tasks::*;

pub type SharedState = Arc<AppState>;

//! Core functionality for project management
//!
//! This module contains the core business logic for vcode:
//! - Configuration management (config.rs)
//! - Project CRUD operations (project.rs)
//! - Editor integration (editor.rs)

pub mod config;
pub mod editor;
pub mod project;

// Re-export commonly used items
pub use config::{Config, get_config, init_config, update_config};
pub use editor::{is_vscode_like_editor, open_with_editor};
pub use project::{
    delete_project, get_projects, rename_project, reset_projects, resolve_path, set_project,
};

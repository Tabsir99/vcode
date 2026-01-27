//! vcode - A fast CLI project launcher for your favorite code editor
//!
//! This library provides the core functionality for managing and launching projects.
//! It's organized into several modules:
//!
//! - `core`: Core business logic (config, projects, editor integration)
//! - `scanner`: Project scanning and detection
//! - `ui`: User interface components (logging, table display)
//! - `commands`: Command handlers for CLI operations

pub mod commands;
pub mod core;
pub mod scanner;
pub mod ui;

// Re-export commonly used items for convenience
pub use core::{
    Config, delete_project, get_config, get_projects, init_config, open_with_editor,
    rename_project, reset_projects, resolve_path, set_project,
};
pub use scanner::{
    FilterMode, FoundProject, ProjectType, add_projects, detect_project_type,
    interactive_select_projects, is_project_directory, scan_projects,
};
pub use ui::{LogType, log, print_table};

pub const APP_NAME: &str = "vcode";

pub mod bulk;
pub mod config;
pub mod detector;
pub mod editor;
pub mod logger;
pub mod project;

pub use bulk::{add_projects, interactive_select_projects, scan_projects, FilterMode, FoundProject};
pub use config::{get_config, init_config, Config};
pub use detector::{detect_project_type, is_project_directory, ProjectType};
pub use editor::open_with_editor;
pub use logger::{log, LogType};
pub use project::{delete_project, get_projects, rename_project, reset_projects, set_project};

pub const APP_NAME: &str = "vcode";

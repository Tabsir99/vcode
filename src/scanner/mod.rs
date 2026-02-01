//! Project scanning and detection
//!
//! This module provides functionality for:
//! - Detecting project types from marker files (detector.rs)
//! - Scanning directories to find projects (scanner.rs)
//! - Bulk project operations

pub mod detector;
pub mod scanner;

// Re-export commonly used items
pub use detector::{ProjectType, detect_project_type, is_project_directory};
pub use scanner::{
    DirectoryMatch, FilterMode, FoundProject, add_projects, interactive_select_projects,
    scan_projects, search_directory_by_name,
};

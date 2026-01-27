//! Project scanning and bulk operations
//!
//! This module provides functionality to:
//! - Scan directories recursively to find projects
//! - Filter directories (skip build artifacts, dependencies, etc.)
//! - Detect project types using marker files
//! - Interactively select projects to add
//! - Add multiple projects at once

use super::detector::{ProjectType, detect_project_type};
use crate::core::project::set_project;
use crate::ui::logger::{LogType, log};
use dialoguer::{MultiSelect, theme::ColorfulTheme};
use std::collections::HashSet;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

// =============================================================================
// Constants
// =============================================================================

/// Directories to skip during scanning (build artifacts, dependencies, etc.)
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".git",
    ".svn",
    ".hg",
    "target",
    "out",
    "bin",
    "build",
    "cmake-build-debug",
    "cmake-build-release",
    "Debug",
    "Release",
    "dist",
    ".next",
    ".nuxt",
    ".angular",
    ".cache",
    ".idea",
    ".vscode",
    "coverage",
    "logs",
    "tmp",
    "temp",
];

/// Represents a project found during directory scanning
#[derive(Debug, Clone)]
pub struct FoundProject {
    pub name: String,
    pub path: PathBuf,
    pub project_type: Option<ProjectType>,
}

impl FoundProject {
    /// Returns a formatted display name with project type
    pub fn display_name(&self) -> String {
        match &self.project_type {
            Some(ptype) => format!("{} ({})", self.name, ptype.name()),
            None => format!("{} (Unknown)", self.name),
        }
    }
}

/// Filter mode for project scanning
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    /// Only include directories that are detected as projects
    Auto,
    /// Include all directories (regardless of project markers)
    All,
}

// =============================================================================
// Public API - Scanning
// =============================================================================

/// Scans a directory recursively to find projects
///
/// # Arguments
/// * `base_path` - Directory to start scanning from
/// * `target_depth` - How many levels deep to scan (1 = immediate children only)
/// * `filter_mode` - Whether to detect projects automatically or include all directories
///
/// # Returns
/// Vector of found projects, or error if base path is invalid
///
/// # Example
/// ```ignore
/// let projects = scan_projects(Path::new("/home/user/projects"), 1, FilterMode::Auto)?;
/// ```
pub fn scan_projects(
    base_path: &Path,
    target_depth: u32,
    filter_mode: FilterMode,
) -> Result<Vec<FoundProject>, Box<dyn std::error::Error>> {
    if !base_path.exists() || !base_path.is_dir() {
        return Err("Base path doesn't exist or is not a directory".into());
    }

    let mut found_projects = Vec::new();
    traverse_and_collect(base_path, target_depth, 1, &mut found_projects, filter_mode)?;

    Ok(found_projects)
}

/// Checks if a directory should be skipped during scanning
pub fn should_skip_dir(dir_name: &str) -> bool {
    SKIP_DIRS.contains(&dir_name)
}

// =============================================================================
// Public API - Selection & Addition
// =============================================================================

/// Presents an interactive multi-select interface for choosing projects
///
/// # Arguments
/// * `projects` - List of found projects to choose from
///
/// # Returns
/// Vector of selected projects, or error if cancelled/failed
pub fn interactive_select_projects(
    projects: Vec<FoundProject>,
) -> Result<Vec<FoundProject>, Box<dyn std::error::Error>> {
    if projects.is_empty() {
        return Ok(vec![]);
    }

    let options: Vec<String> = projects.iter().map(|p| p.display_name()).collect();
    let defaults: Vec<bool> = vec![true; options.len()];

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select projects to add (Space to toggle, Enter to confirm)")
        .items(&options)
        .defaults(&defaults)
        .interact_opt()?;

    match selections {
        Some(indices) => {
            let selected: Vec<FoundProject> =
                indices.into_iter().map(|i| projects[i].clone()).collect();
            Ok(selected)
        }
        None => Err("Selection cancelled".into()),
    }
}

/// Adds multiple projects to the project registry
///
/// # Arguments
/// * `projects` - Projects to add
///
/// # Returns
/// Number of successfully added projects
pub fn add_projects(projects: Vec<FoundProject>) -> Result<usize, Box<dyn std::error::Error>> {
    let mut added_count = 0;

    for project in projects {
        match set_project(&project.name, project.path.to_str().unwrap()) {
            Ok(()) => {
                added_count += 1;
                log(&format!("  + {}", project.display_name()), LogType::Normal);
            }
            Err(_) => {
                log(
                    &format!("  ⚠ Failed to add: {}", project.name),
                    LogType::Warning,
                );
            }
        }
    }

    Ok(added_count)
}

// =============================================================================
// Private Implementation
// =============================================================================

/// Recursively traverses directories and collects projects
fn traverse_and_collect(
    current_path: &Path,
    target_depth: u32,
    current_depth: u32,
    found_projects: &mut Vec<FoundProject>,
    filter_mode: FilterMode,
) -> Result<(), Box<dyn std::error::Error>> {
    if current_depth > target_depth {
        return Ok(());
    }

    let skip_dirs: HashSet<&str> = SKIP_DIRS.iter().copied().collect();
    let entries = read_dir(current_path)?;

    for entry in entries {
        let path = entry?.path();

        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                // Skip directories that are known build artifacts, dependencies, etc.
                if skip_dirs.contains(dir_name) {
                    continue;
                }

                if current_depth == target_depth {
                    // We've reached target depth - check if this is a project
                    let project_type = detect_project_type(&path);

                    let should_include = match filter_mode {
                        FilterMode::Auto => project_type.is_some(),
                        FilterMode::All => true,
                    };

                    if should_include {
                        found_projects.push(FoundProject {
                            name: dir_name.to_string(),
                            path: path.clone(),
                            project_type,
                        });
                    }
                } else {
                    // Haven't reached target depth yet - recurse deeper
                    traverse_and_collect(
                        &path,
                        target_depth,
                        current_depth + 1,
                        found_projects,
                        filter_mode,
                    )?;
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_projects_auto_mode() {
        let temp_dir = TempDir::new().unwrap();

        let rust_project = temp_dir.path().join("my-rust-project");
        fs::create_dir(&rust_project).unwrap();
        fs::write(rust_project.join("Cargo.toml"), "[package]").unwrap();

        let random_dir = temp_dir.path().join("random-folder");
        fs::create_dir(&random_dir).unwrap();

        let found = scan_projects(temp_dir.path(), 1, FilterMode::Auto).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "my-rust-project");
        assert_eq!(found[0].project_type, Some(ProjectType::Rust));
    }

    #[test]
    fn test_scan_projects_all_mode() {
        let temp_dir = TempDir::new().unwrap();

        let rust_project = temp_dir.path().join("my-rust-project");
        fs::create_dir(&rust_project).unwrap();
        fs::write(rust_project.join("Cargo.toml"), "[package]").unwrap();

        let random_dir = temp_dir.path().join("random-folder");
        fs::create_dir(&random_dir).unwrap();

        let found = scan_projects(temp_dir.path(), 1, FilterMode::All).unwrap();

        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_skip_dirs() {
        let temp_dir = TempDir::new().unwrap();

        let node_modules = temp_dir.path().join("node_modules");
        fs::create_dir(&node_modules).unwrap();

        let found = scan_projects(temp_dir.path(), 1, FilterMode::All).unwrap();

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_should_skip_dir() {
        assert!(should_skip_dir("node_modules"));
        assert!(should_skip_dir("target"));
        assert!(should_skip_dir(".git"));
        assert!(!should_skip_dir("my-project"));
    }
}

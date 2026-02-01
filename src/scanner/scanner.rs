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
use std::fs::read_dir;
use std::path::{Path, PathBuf};

// =============================================================================
// Constants
// =============================================================================

/// Directories to always skip (build artifacts, dependencies, system dirs)
const SKIP_DIRS: &[&str] = &[
    // Build artifacts & dependencies
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
    // Package manager caches
    ".npm",
    ".cargo",
    ".rustup",
    ".nvm",
    ".pyenv",
    ".rbenv",
    ".gradle",
    ".m2",
    ".maven",
    // Tooling
    ".docker",
    ".kube",
    ".minikube",
    ".vagrant",
    ".ansible",
    ".terraform",
    ".pulumi",
    // System
    ".gnupg",
    ".ssh",
    ".pki",
    ".mozilla",
    ".thunderbird",
    ".wine",
    ".steam",
    ".local",
    ".config",
    ".var",
    ".Trash",
    "snap",
    "go",
    ".go",
    // Common non-project dirs
    "Library",
    "Pictures",
    "Music",
    "Videos",
    "Movies",
    "Downloads",
    "Documents",
    "Desktop",
    "Public",
    "Templates",
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
// Public API - Directory Search
// =============================================================================

/// Result of a directory name search
#[derive(Debug, Clone)]
pub struct DirectoryMatch {
    pub name: String,
    pub path: PathBuf,
}

/// Searches the filesystem for directories matching the given name
///
/// Starts from the user's home directory and common project locations.
/// Skips known build/dependency directories (node_modules, target, etc.)
///
/// # Arguments
/// * `dir_name` - The directory name to search for (case-insensitive)
///
/// # Returns
/// Vector of matching directories, sorted by path length (shortest first)
pub fn search_directory_by_name(dir_name: &str) -> Result<Vec<DirectoryMatch>, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;

    let mut matches = Vec::new();
    let target_name = dir_name.to_lowercase();

    // Search from home directory with reasonable depth
    search_recursive(&home, &target_name, 0, 6, &mut matches);

    // Sort by path length (prefer shallower matches)
    matches.sort_by(|a, b| {
        let depth_a = a.path.components().count();
        let depth_b = b.path.components().count();
        depth_a.cmp(&depth_b)
    });

    // Limit results to avoid overwhelming the user
    matches.truncate(20);

    Ok(matches)
}

fn search_recursive(
    current_path: &Path,
    target_name: &str,
    current_depth: u32,
    max_depth: u32,
    matches: &mut Vec<DirectoryMatch>,
) {
    if current_depth > max_depth {
        return;
    }

    let entries = match read_dir(current_path) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Skip hidden directories after root level
        if dir_name.starts_with('.') && current_depth > 0 {
            continue;
        }

        if should_skip_dir(dir_name) {
            continue;
        }

        if dir_name.to_lowercase() == target_name {
            matches.push(DirectoryMatch {
                name: dir_name.to_string(),
                path: path.clone(),
            });
        }

        if current_depth < max_depth {
            search_recursive(&path, target_name, current_depth + 1, max_depth, matches);
        }
    }
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

    for entry in read_dir(current_path)? {
        let path = entry?.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        if should_skip_dir(dir_name) {
            continue;
        }

        if current_depth == target_depth {
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
            traverse_and_collect(&path, target_depth, current_depth + 1, found_projects, filter_mode)?;
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

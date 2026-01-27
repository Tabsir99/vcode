use crate::detector::{detect_project_type, ProjectType};
use crate::logger::{log, LogType};
use crate::project::set_project;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use std::collections::HashSet;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[
    // JavaScript / Node
    "node_modules",
    // Python
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    // VCS
    ".git",
    ".svn",
    ".hg",
    // Rust / Cargo
    "target",
    // Java / JVM
    "out",
    "bin",
    "build",
    // C / C++ / general build systems
    "cmake-build-debug",
    "cmake-build-release",
    "Debug",
    "Release",
    // Web / Frontend
    "dist",
    ".next",
    ".nuxt",
    ".angular",
    ".cache",
    // IDE / Editor stuff
    ".idea",
    ".vscode",
    // Other common generated dirs
    "coverage",
    "logs",
    "tmp",
    "temp",
];

#[derive(Debug, Clone)]
pub struct FoundProject {
    pub name: String,
    pub path: PathBuf,
    pub project_type: Option<ProjectType>,
}

impl FoundProject {
    pub fn display_name(&self) -> String {
        match &self.project_type {
            Some(ptype) => format!("{} ({})", self.name, ptype.name()),
            None => format!("{} (Unknown)", self.name),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    Auto,
    All,
}

pub fn scan_projects(
    base_path: &Path,
    target_depth: u32,
    filter_mode: FilterMode,
) -> Result<Vec<FoundProject>, Box<dyn std::error::Error>> {
    if !base_path.exists() || !base_path.is_dir() {
        return Err("Base path doesn't exist or is not a directory".into());
    }

    let mut found_projects = Vec::new();
    traverse_and_collect(
        base_path,
        target_depth,
        1,
        &mut found_projects,
        filter_mode,
    )?;

    Ok(found_projects)
}

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
                if skip_dirs.contains(dir_name) {
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
            let selected: Vec<FoundProject> = indices
                .into_iter()
                .map(|i| projects[i].clone())
                .collect();
            Ok(selected)
        }
        None => Err("Selection cancelled".into()),
    }
}

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

pub fn should_skip_dir(dir_name: &str) -> bool {
    SKIP_DIRS.contains(&dir_name)
}

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
}

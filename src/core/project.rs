use crate::APP_NAME;
use dirs;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;

pub fn get_data_path() -> PathBuf {
    dirs::data_dir()
        .expect("Could not find data directory")
        .join(APP_NAME)
        .join("projects.json")
}

pub fn get_projects() -> HashMap<String, String> {
    let data_dir = dirs::data_dir()
        .expect("Could not find data directory")
        .join(APP_NAME);
    let data_path = data_dir.join("projects.json");

    if !data_dir.exists() {
        create_dir_all(&data_dir).expect("Failed to create data directory");
    }

    if !data_path.exists() {
        write(
            &data_path,
            serde_json::to_string_pretty(&HashMap::<String, String>::new()).unwrap(),
        )
        .expect("Failed to create data json");
    }

    serde_json::from_str(&read_to_string(&data_path).unwrap())
        .expect("Failed to parse projects.json")
}

pub fn set_project(name: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut projects = get_projects();
    projects.insert(name.to_string(), path.to_string());
    write_projects(&projects)
}

/// Persist the entire project map in one write. Used by batch operations
/// (prune, future imports) to avoid N rewrites of the same JSON file.
pub fn write_projects(projects: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(projects)?;
    write(get_data_path(), json)?;
    Ok(())
}

/// Resolves `raw_path`, verifies it's an existing directory, and inserts/updates
/// the project entry. Returns the resolved canonical path on success, or a
/// human-readable error explaining what went wrong.
///
/// Centralizes the resolve+validate+write pattern shared by `add`, `update`,
/// and other future write operations so they don't drift in their error
/// messages or validation rules.
pub fn set_project_validated(name: &str, raw_path: &str) -> Result<PathBuf, String> {
    let resolved = resolve_path(raw_path);
    if !resolved.exists() {
        return Err(format!("Path does not exist: {}", resolved.display()));
    }
    if !resolved.is_dir() {
        return Err(format!("Path is not a directory: {}", resolved.display()));
    }
    let path_str = resolved
        .to_str()
        .ok_or_else(|| format!("Path contains invalid UTF-8: {}", resolved.display()))?;
    set_project(name, path_str).map_err(|e| format!("Failed to write registry: {}", e))?;
    Ok(resolved)
}

/// Extracts the final path segment as an owned String, or the lossy full path
/// if no basename can be determined (e.g. root `/`).
pub fn path_basename(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

pub fn delete_project(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut projects = get_projects();
    projects.remove(name);
    write_projects(&projects)
}

pub fn rename_project(old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut projects = get_projects();

    if let Some(path) = projects.remove(old_name) {
        projects.insert(new_name.to_string(), path);
        write_projects(&projects)
    } else {
        Err(format!("Project '{}' not found", old_name).into())
    }
}

pub fn reset_projects() -> Result<(), Box<dyn std::error::Error>> {
    write(get_data_path(), "{}")?;
    Ok(())
}

pub fn resolve_path(input: &str) -> PathBuf {
    let expanded = expand_tilde(input);
    let path = PathBuf::from(&expanded);

    if path.exists() {
        std::fs::canonicalize(&path).expect("Failed to canonicalize path")
    } else if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .expect("Failed to get current dir")
            .join(path)
    }
}

/// Expands a leading `~` or `~/` to the user's home directory.
/// Leaves the input unchanged if it doesn't start with `~`.
fn expand_tilde(input: &str) -> String {
    if input == "~" {
        return dirs::home_dir()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|| input.to_string());
    }
    if let (Some(rest), Some(home)) = (input.strip_prefix("~/"), dirs::home_dir()) {
        return home.join(rest).to_string_lossy().into_owned();
    }
    input.to_string()
}

/// Returns the resolved path if `input` refers to an existing directory, else None.
/// Used by commands that want to fall back to opening a path when a project
/// name lookup fails (e.g. `vcode .`, `vcode ../foo`, `vcode ~/work/x`).
pub fn try_resolve_existing_dir(input: &str) -> Option<PathBuf> {
    let resolved = resolve_path(input);
    if resolved.is_dir() { Some(resolved) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_resolve_path_current_dir() {
        let current = env::current_dir().unwrap();
        let resolved = resolve_path(".");
        assert_eq!(resolved, current);
    }

    #[test]
    fn test_resolve_path_relative() {
        let current = env::current_dir().unwrap();
        let expected = current.join("test_path");
        let resolved = resolve_path("test_path");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_get_data_path() {
        let path = get_data_path();
        assert!(path.to_string_lossy().contains("vcode"));
        assert!(path.to_string_lossy().ends_with("projects.json"));
    }

    #[test]
    fn test_expand_tilde_bare() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_tilde("~"), home.to_string_lossy());
    }

    #[test]
    fn test_expand_tilde_with_path() {
        let home = dirs::home_dir().unwrap();
        let expected = home.join("foo/bar").to_string_lossy().into_owned();
        assert_eq!(expand_tilde("~/foo/bar"), expected);
    }

    #[test]
    fn test_expand_tilde_noop() {
        assert_eq!(expand_tilde("/abs/path"), "/abs/path");
        assert_eq!(expand_tilde("./rel"), "./rel");
        assert_eq!(expand_tilde("name"), "name");
    }

    #[test]
    fn test_try_resolve_existing_dir_cwd() {
        let resolved = try_resolve_existing_dir(".").unwrap();
        assert_eq!(resolved, env::current_dir().unwrap());
    }

    #[test]
    fn test_try_resolve_existing_dir_missing() {
        assert!(try_resolve_existing_dir("/no/such/path/should/exist/here").is_none());
    }
}

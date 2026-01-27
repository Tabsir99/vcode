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

    let json = serde_json::to_string_pretty(&projects)?;
    write(get_data_path(), json)?;
    Ok(())
}

pub fn delete_project(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut projects = get_projects();
    projects.remove(name);

    let json = serde_json::to_string_pretty(&projects)?;
    write(get_data_path(), json)?;
    Ok(())
}

pub fn rename_project(old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut projects = get_projects();

    if let Some(path) = projects.remove(old_name) {
        projects.insert(new_name.to_string(), path);
        let json = serde_json::to_string_pretty(&projects)?;
        write(get_data_path(), json)?;
        Ok(())
    } else {
        Err(format!("Project '{}' not found", old_name).into())
    }
}

pub fn reset_projects() -> Result<(), Box<dyn std::error::Error>> {
    write(get_data_path(), "{}")?;
    Ok(())
}

pub fn resolve_path(input: &str) -> PathBuf {
    let path = PathBuf::from(input);

    if path.exists() {
        std::fs::canonicalize(&path).expect("Failed to canonicalize path")
    } else {
        std::env::current_dir()
            .expect("Failed to get current dir")
            .join(path)
    }
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
}

use crate::core::{
    config::{Config, get_config, update_config},
    editor::open_with_editor,
    project::{
        delete_project, get_projects, rename_project, reset_projects, resolve_path, set_project,
    },
};
use crate::scanner::{FilterMode, add_projects, interactive_select_projects, scan_projects};
use crate::ui::{LogType, log, print_table};
use std::path::PathBuf;

pub fn handle_add(name: String, path: String) {
    match set_project(&name, &resolve_path(&path).to_str().unwrap()) {
        Ok(()) => log(&format!("✓ Added project '{}'", name), LogType::Success),
        Err(_e) => log("✗ Failed to add project", LogType::Error),
    };
}

pub fn handle_remove(name: String) {
    match delete_project(&name) {
        Ok(()) => log(&format!("✓ Removed project '{}'", name), LogType::Success),
        Err(_) => log(&format!("✗ Project '{}' not found", name), LogType::Error),
    }
}

pub fn handle_list(json: bool, interactive: bool, reuse: bool, editor_override: Option<String>) {
    let projects = get_projects();

    if json {
        println!("{}", serde_json::to_string_pretty(&projects).unwrap());
        return;
    }

    if interactive {
        if projects.is_empty() {
            log(
                "No projects found. Add one with: vcode add <name> <path>",
                LogType::Info,
            );
            return;
        }

        use inquire::Select;
        let mut sorted: Vec<_> = projects.iter().collect();
        sorted.sort_by_key(|(name, _)| name.to_lowercase());

        let options: Vec<String> = sorted
            .iter()
            .map(|(name, path)| format!("{} → {}", name, path))
            .collect();

        match Select::new("Select a project to open:", options)
            .with_page_size(15)
            .prompt()
        {
            Ok(selected) => {
                let project_name = selected.split(" → ").next().unwrap();
                let config = get_config();
                let editor = editor_override.as_deref().unwrap_or(&config.default_editor);

                if let Some(path) = projects.get(project_name) {
                    match open_with_editor(editor, path, reuse) {
                        Ok(()) => {
                            log(
                                &format!("Opening '{}' in {}", project_name, editor),
                                LogType::Success,
                            );
                        }
                        Err(e) => {
                            log(&format!("✗ Failed to open project: {}", e), LogType::Error);
                        }
                    }
                }
            }
            Err(_) => {
                log("Selection cancelled", LogType::Info);
            }
        }
    } else {
        print_table(&projects);
    }
}

pub fn handle_search(query: String) {
    let projects = get_projects();
    let query_lower = query.to_lowercase();
    let filtered: std::collections::HashMap<_, _> = projects
        .iter()
        .filter(|(name, path)| {
            name.to_lowercase().contains(&query_lower) || path.to_lowercase().contains(&query_lower)
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if filtered.is_empty() {
        log(
            &format!("No projects found matching '{}'", query),
            LogType::Info,
        );
    } else {
        log(&format!("Projects matching '{}':", query), LogType::Info);
        print_table(&filtered);
    }
}

pub fn handle_rename(old_name: String, new_name: String) {
    match rename_project(&old_name, &new_name) {
        Ok(()) => log(
            &format!("✓ Renamed '{}' → '{}'", old_name, new_name),
            LogType::Success,
        ),
        Err(e) => log(&format!("✗ {}", e), LogType::Error),
    }
}

pub fn handle_scan(path: Option<String>, depth: u32, filter: String, no_review: bool) {
    let config = get_config();
    let base_path = match path {
        Some(p) => resolve_path(&p),
        None => PathBuf::from(&config.projects_root),
    };

    let filter_mode = match filter.to_lowercase().as_str() {
        "all" => FilterMode::All,
        "auto" => FilterMode::Auto,
        _ => {
            log("Invalid filter mode. Use 'auto' or 'all'", LogType::Error);
            return;
        }
    };

    log(
        &format!(
            "Scanning {} at depth {} (filter: {})...",
            base_path.display(),
            depth,
            filter
        ),
        LogType::Info,
    );

    match scan_projects(&base_path, depth, filter_mode) {
        Ok(found_projects) => {
            if found_projects.is_empty() {
                log("No projects found", LogType::Info);
                return;
            }

            let project_count = found_projects.len();

            log(
                &format!(
                    "✓ Found {} project{}",
                    project_count,
                    if project_count == 1 { "" } else { "s" }
                ),
                LogType::Success,
            );

            let projects_to_add = if no_review {
                found_projects
            } else {
                match interactive_select_projects(found_projects) {
                    Ok(selected) => selected,
                    Err(_) => {
                        log("Scan cancelled", LogType::Info);
                        return;
                    }
                }
            };

            if projects_to_add.is_empty() {
                log("No projects selected", LogType::Info);
                return;
            }

            match add_projects(projects_to_add) {
                Ok(added_count) => {
                    log(
                        &format!(
                            "\n✓ Added {} project{}",
                            added_count,
                            if added_count == 1 { "" } else { "s" }
                        ),
                        LogType::Success,
                    );
                }
                Err(e) => {
                    log(&format!("✗ Failed to add projects: {}", e), LogType::Error);
                }
            }
        }
        Err(e) => {
            log(&format!("✗ Failed to scan: {}", e), LogType::Error);
        }
    }
}

pub fn handle_config(show: bool, projects_root: Option<String>, editor: Option<String>) {
    let config = get_config();

    if show || (projects_root.is_none() && editor.is_none()) {
        log("Configuration:", LogType::Info);
        println!("\n  Editor:        {}", config.default_editor);
        println!("  Projects Root: {}\n", config.projects_root);
    } else {
        let updated_config = Config {
            projects_root: projects_root.unwrap_or(config.projects_root),
            default_editor: editor.unwrap_or(config.default_editor),
        };

        update_config(&updated_config).expect("Failed to update config");
        log("✓ Configuration updated", LogType::Success);
    }
}

pub fn handle_clear(yes: bool) {
    if !yes {
        use inquire::Confirm;
        let confirm = Confirm::new("Are you sure you want to clear all projects?")
            .with_default(false)
            .prompt();

        match confirm {
            Ok(true) => {}
            Ok(false) => {
                log("Cancelled", LogType::Info);
                return;
            }
            Err(_) => return,
        }
    }

    match reset_projects() {
        Ok(()) => log("✓ All projects cleared", LogType::Success),
        Err(_) => log("✗ Failed to clear projects", LogType::Error),
    }
}

pub fn handle_open_project(project_name: String, reuse: bool, editor_override: Option<String>) {
    let projects = get_projects();
    let path = projects.get(&project_name);
    let config = get_config();
    let editor = editor_override.as_deref().unwrap_or(&config.default_editor);

    match path {
        None => {
            log(
                &format!("✗ Project '{}' not found", project_name),
                LogType::Error,
            );
            log(
                "\nTip: Use 'vcode list' to see all projects or 'vcode add' to add a new one",
                LogType::Info,
            );
            std::process::exit(1);
        }
        Some(path) => match open_with_editor(editor, path, reuse) {
            Ok(()) => {
                log(
                    &format!("Opening '{}' in {}", project_name, editor),
                    LogType::Success,
                );
                std::process::exit(0);
            }
            Err(e) => {
                log(&format!("✗ Failed to open project: {}", e), LogType::Error);
                std::process::exit(1);
            }
        },
    }
}

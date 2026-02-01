use crate::core::{
    config::{EditorConfig, get_config, reset_config, update_config},
    editor::open_with_editor,
    project::{
        delete_project, get_projects, rename_project, reset_projects, resolve_path, set_project,
    },
};
use crate::scanner::{FilterMode, add_projects, interactive_select_projects, scan_projects, search_directory_by_name};
use crate::ui::{LogType, log, print_table};
use clap::Subcommand;
use colored::Colorize;
use std::path::PathBuf;

/// Config subcommand actions
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set a configuration value (key: editor, projects-root)
    Set {
        /// Key to set
        key: String,
        /// Value to set
        value: String,
    },
    /// List all registered editors
    Editors,
    /// Add a custom editor
    Add,
    /// Remove an editor
    Remove {
        /// Editor name to remove
        name: String,
    },
    /// Interactive configuration wizard
    Edit,
    /// Reset configuration to defaults
    Reset,
}

pub fn handle_add(name: String, path: Option<String>, find: bool) {
    if find {
        handle_find_add(name);
    } else {
        match path {
            Some(p) => {
                match set_project(&name, &resolve_path(&p).to_str().unwrap()) {
                    Ok(()) => log(&format!("✓ Added project '{}'", name), LogType::Success),
                    Err(_e) => log("✗ Failed to add project", LogType::Error),
                };
            }
            None => {
                log("✗ Path is required when not using --find", LogType::Error);
                log("Usage: vcode add <name> <path>", LogType::Info);
                log("   or: vcode add <name> --find", LogType::Info);
            }
        }
    }
}

fn handle_find_add(name: String) {
    use inquire::Select;

    log(&format!("Searching for '{}'...", name), LogType::Info);

    match search_directory_by_name(&name) {
        Ok(matches) => {
            if matches.is_empty() {
                log(&format!("✗ No directory named '{}' found", name), LogType::Error);
                return;
            }

            if matches.len() == 1 {
                let found = &matches[0];
                log(
                    &format!("✓ Found: {}", found.path.display()),
                    LogType::Success,
                );

                match set_project(&found.name, found.path.to_str().unwrap()) {
                    Ok(()) => log(&format!("✓ Added project '{}'", found.name), LogType::Success),
                    Err(_) => log("✗ Failed to add project", LogType::Error),
                }
                return;
            }

            // Multiple matches - let user select
            log(
                &format!("Found {} matches:", matches.len()),
                LogType::Info,
            );
            println!();

            let options: Vec<String> = matches
                .iter()
                .map(|m| format!("{} → {}", m.name, m.path.display()))
                .collect();

            match Select::new("Select directory to add:", options)
                .with_page_size(10)
                .prompt()
            {
                Ok(selected) => {
                    let idx = matches
                        .iter()
                        .position(|m| format!("{} → {}", m.name, m.path.display()) == selected)
                        .unwrap();
                    let chosen = &matches[idx];

                    match set_project(&chosen.name, chosen.path.to_str().unwrap()) {
                        Ok(()) => {
                            log(&format!("✓ Added project '{}'", chosen.name), LogType::Success)
                        }
                        Err(_) => log("✗ Failed to add project", LogType::Error),
                    }
                }
                Err(_) => {
                    log("Selection cancelled", LogType::Info);
                }
            }
        }
        Err(e) => {
            log(&format!("✗ Search failed: {}", e), LogType::Error);
        }
    }
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

pub fn handle_config(action: Option<ConfigAction>) {
    match action {
        None => config_show(),
        Some(ConfigAction::Show) => config_show(),
        Some(ConfigAction::Set { key, value }) => config_set(&key, &value),
        Some(ConfigAction::Editors) => config_editors(),
        Some(ConfigAction::Add) => config_add_editor(),
        Some(ConfigAction::Remove { name }) => config_remove_editor(&name),
        Some(ConfigAction::Edit) => config_edit(),
        Some(ConfigAction::Reset) => config_reset(),
    }
}

fn config_show() {
    let config = get_config();

    println!();
    println!("  {}", "┌──────────────────────────────────────────────────┐".dimmed());
    println!(
        "  {}  {}  {}",
        "│".dimmed(),
        format!("{:<10}", "Editor").cyan().bold(),
        config.default_editor.white()
    );
    println!(
        "  {}  {}  {}",
        "│".dimmed(),
        format!("{:<10}", "Projects").cyan().bold(),
        config.projects_root.white()
    );
    println!("  {}", "└──────────────────────────────────────────────────┘".dimmed());
    println!();
    println!(
        "  {}",
        format!("{} editors  →  vcode config editors", config.editors.len()).dimmed()
    );
    println!();
}

fn config_set(key: &str, value: &str) {
    let mut config = get_config();

    match key {
        "editor" => {
            if !config.editors.contains_key(value) {
                log(&format!("✗ Unknown editor '{}'. Use 'vcode config editors' to see available options.", value), LogType::Error);
                return;
            }
            config.default_editor = value.to_string();
        }
        "projects-root" => {
            let path = resolve_path(value);
            if !path.exists() {
                log(&format!("✗ Path does not exist: {}", path.display()), LogType::Error);
                return;
            }
            config.projects_root = path.to_string_lossy().to_string();
        }
        _ => {
            log(&format!("✗ Unknown key '{}'. Valid keys: editor, projects-root", key), LogType::Error);
            return;
        }
    }

    update_config(&config).expect("Failed to update config");
    log(&format!("✓ Set {} = {}", key, value), LogType::Success);
}

fn config_editors() {
    let config = get_config();

    println!();
    println!("  ┌─────────────────────────────────────────────────────────┐");
    println!("  │  Registered Editors                                     │");
    println!("  └─────────────────────────────────────────────────────────┘");
    println!();

    let mut editors: Vec<_> = config.editors.iter().collect();
    editors.sort_by_key(|(name, _)| name.to_lowercase());

    let max_name_len = editors.iter().map(|(n, _)| n.len()).max().unwrap_or(8);

    for (name, editor_config) in editors {
        let is_default = name == &config.default_editor;
        let marker = if is_default { " ←" } else { "" };

        let args_display = if editor_config.args.is_empty() {
            String::new()
        } else {
            format!("  [{}]", editor_config.args.join(" "))
        };

        println!(
            "  {:<width$}  {}{}{}",
            name,
            editor_config.command,
            args_display,
            marker,
            width = max_name_len
        );
    }

    println!();
    println!("  ← indicates default editor");
    println!();
    println!("  To change default: vcode config set editor <name>");
    println!("  To add new:        vcode config add");
    println!();
}

fn config_add_editor() {
    use inquire::{Confirm, Text};

    let mut config = get_config();

    println!();
    println!("  Add Custom Editor");
    println!("  ────────────────────────────────────────");
    println!();

    let name = match Text::new("  Name (e.g., helix, lapce):").prompt() {
        Ok(n) if !n.trim().is_empty() => n.trim().to_lowercase(),
        _ => {
            println!();
            log("Cancelled", LogType::Info);
            return;
        }
    };

    if config.editors.contains_key(&name) {
        println!();
        log(&format!("Editor '{}' already exists", name), LogType::Warning);
        let confirm = Confirm::new("  Overwrite?")
            .with_default(false)
            .prompt();
        if !matches!(confirm, Ok(true)) {
            return;
        }
    }

    let command = match Text::new("  Command (e.g., hx, /usr/bin/helix):")
        .with_default(&name)
        .prompt()
    {
        Ok(c) => c,
        _ => {
            println!();
            log("Cancelled", LogType::Info);
            return;
        }
    };

    let args_str = Text::new("  Arguments (space-separated, or empty):")
        .with_default("")
        .prompt()
        .unwrap_or_default();

    let args: Vec<String> = if args_str.trim().is_empty() {
        Vec::new()
    } else {
        args_str.split_whitespace().map(|s| s.to_string()).collect()
    };

    let reuse_flag = Text::new("  Reuse window flag (e.g., -r, or empty):")
        .with_default("")
        .prompt()
        .ok()
        .filter(|s| !s.trim().is_empty());

    let editor_config = EditorConfig {
        command,
        args,
        reuse_flag,
    };

    config.add_editor(name.clone(), editor_config);
    update_config(&config).expect("Failed to update config");

    println!();
    log(&format!("✓ Added editor '{}'", name), LogType::Success);
}

fn config_remove_editor(name: &str) {
    let mut config = get_config();

    if name == config.default_editor {
        log("✗ Cannot remove the default editor. Change it first with: vcode config set editor <other>", LogType::Error);
        return;
    }

    if config.remove_editor(name) {
        update_config(&config).expect("Failed to update config");
        log(&format!("✓ Removed editor '{}'", name), LogType::Success);
    } else {
        log(&format!("✗ Editor '{}' not found", name), LogType::Error);
    }
}

fn config_edit() {
    use inquire::Select;

    println!();
    println!("  ┌─────────────────────────────────────────────────────────┐");
    println!("  │  Configuration Wizard                                   │");
    println!("  └─────────────────────────────────────────────────────────┘");
    println!();

    let mut config = get_config();

    let options = vec![
        "Set default editor",
        "Set projects root",
        "Add custom editor",
        "Remove editor",
        "Exit",
    ];

    loop {
        match Select::new("  What would you like to do?", options.clone()).prompt() {
            Ok("Set default editor") => {
                let editor_names: Vec<&str> = config.editors.keys().map(|s| s.as_str()).collect();
                if let Ok(selected) = Select::new("  Select editor:", editor_names).prompt() {
                    config.default_editor = selected.to_string();
                    update_config(&config).expect("Failed to update config");
                    println!();
                    log(&format!("✓ Default editor: {}", selected), LogType::Success);
                }
            }
            Ok("Set projects root") => {
                use inquire::Text;
                if let Ok(path) = Text::new("  Projects directory:")
                    .with_default(&config.projects_root)
                    .prompt()
                {
                    config.projects_root = resolve_path(&path).to_string_lossy().to_string();
                    update_config(&config).expect("Failed to update config");
                    println!();
                    log("✓ Projects root updated", LogType::Success);
                }
            }
            Ok("Add custom editor") => {
                config_add_editor();
                config = get_config();
            }
            Ok("Remove editor") => {
                let editor_names: Vec<String> = config.editors.keys().cloned().collect();
                let editor_refs: Vec<&str> = editor_names.iter().map(|s| s.as_str()).collect();
                if let Ok(selected) = Select::new("  Select editor to remove:", editor_refs).prompt() {
                    if selected == config.default_editor {
                        println!();
                        log("✗ Cannot remove the default editor", LogType::Error);
                    } else {
                        config.remove_editor(selected);
                        update_config(&config).expect("Failed to update config");
                        println!();
                        log(&format!("✓ Removed '{}'", selected), LogType::Success);
                    }
                }
            }
            Ok("Exit") | Err(_) => {
                println!();
                break;
            }
            _ => {}
        }
        println!();
    }
}

fn config_reset() {
    use inquire::Confirm;

    println!();
    log("This will reset all settings to defaults.", LogType::Warning);
    println!();

    let confirm = Confirm::new("  Continue?")
        .with_default(false)
        .prompt();

    match confirm {
        Ok(true) => {
            reset_config();
            println!();
            log("✓ Configuration reset", LogType::Success);
        }
        _ => {
            println!();
            log("Cancelled", LogType::Info);
        }
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

use crate::core::{
    config::{EditorConfig, get_config, reset_config, update_config},
    editor::open_with_editor,
    project::{
        delete_project, get_projects, path_basename, rename_project, reset_projects, resolve_path,
        set_project, set_project_validated, try_resolve_existing_dir, write_projects,
    },
};
use crate::scanner::{
    FilterMode, ProjectType, add_projects, detect_project_type, interactive_select_projects,
    scan_projects, search_directory_by_name,
};
use crate::ui::{LogType, log, print_project_rows, print_table};
use clap::{Subcommand, ValueEnum};
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Sort order for `vcode list`. Wired into clap via `ValueEnum` so bad inputs
/// are rejected at parse time and shell completion lists the choices.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Path,
    Type,
}

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
        return;
    }

    let (project_name, raw_path) = match path {
        Some(p) => (name, p),
        None => match infer_name_from_path_arg(&name) {
            Some(inferred) => (inferred, name),
            None => {
                log("✗ Path is required when not using --find", LogType::Error);
                log("Usage: vcode add <name> <path>", LogType::Info);
                log(
                    "   or: vcode add <path>            (name inferred from basename)",
                    LogType::Info,
                );
                log("   or: vcode add <name> --find", LogType::Info);
                std::process::exit(1);
            }
        },
    };

    // Warn (but don't block) when overwriting an existing project. The check
    // happens before write to give the user a heads-up at the right moment.
    let existing = get_projects();
    if let Some(old_path) = existing.get(&project_name) {
        let resolved_str = resolve_path(&raw_path).to_string_lossy().into_owned();
        if old_path != &resolved_str {
            log(
                &format!("⚠ Overwriting '{}' (was: {})", project_name, old_path),
                LogType::Warning,
            );
        }
    }

    match set_project_validated(&project_name, &raw_path) {
        Ok(resolved) => log(
            &format!("✓ Added project '{}' → {}", project_name, resolved.display()),
            LogType::Success,
        ),
        Err(e) => {
            log(&format!("✗ {}", e), LogType::Error);
            std::process::exit(1);
        }
    }
}

/// When the user passes only one positional argument to `add`, decide whether
/// it should be treated as a path (with the name inferred from its basename).
/// Returns the inferred name if so, or None if the argument looks like a bare
/// project name.
fn infer_name_from_path_arg(arg: &str) -> Option<String> {
    if !looks_like_path(arg) && try_resolve_existing_dir(arg).is_none() {
        return None;
    }
    let base = path_basename(&resolve_path(arg));
    if base.is_empty() { None } else { Some(base) }
}

fn looks_like_path(s: &str) -> bool {
    s == "."
        || s == ".."
        || s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with('~')
        || s.starts_with('/')
        || s.contains('/')
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

pub fn handle_list(
    json: bool,
    interactive: bool,
    reuse: bool,
    editor_override: Option<String>,
    sort: SortKey,
    filter: Option<String>,
) {
    let projects = get_projects();

    // Detect project types up front when needed (filter or sort=type) so the
    // expensive marker-file scan runs once per project rather than once per
    // pipeline stage.
    let needs_types = filter.is_some() || sort == SortKey::Type;
    let mut rows: Vec<TypedRow> = projects
        .into_iter()
        .map(|(name, path)| {
            let ty = if needs_types {
                detect_project_type(Path::new(&path))
            } else {
                None
            };
            TypedRow { name, path, ty }
        })
        .collect();

    if let Some(type_filter) = filter.as_deref() {
        let target = type_filter.to_lowercase();
        rows.retain(|r| {
            r.ty.map(|t| t.name().to_lowercase() == target)
                .unwrap_or(false)
        });
    }

    sort_rows(&mut rows, sort);

    if json {
        let map: HashMap<&str, &str> = rows
            .iter()
            .map(|r| (r.name.as_str(), r.path.as_str()))
            .collect();
        println!("{}", serde_json::to_string_pretty(&map).unwrap());
        return;
    }

    if interactive {
        if rows.is_empty() {
            log(
                "No projects found. Add one with: vcode add <name> <path>",
                LogType::Info,
            );
            return;
        }
        run_interactive_open(&rows, reuse, editor_override);
        return;
    }

    let pairs: Vec<(String, String)> = rows.into_iter().map(|r| (r.name, r.path)).collect();
    print_project_rows(&pairs);
}

/// A list row carrying its (optionally detected) project type so filter and
/// sort can share one detection pass per project.
struct TypedRow {
    name: String,
    path: String,
    ty: Option<ProjectType>,
}

fn sort_rows(rows: &mut [TypedRow], sort: SortKey) {
    match sort {
        SortKey::Name => rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        SortKey::Path => rows.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase())),
        SortKey::Type => rows.sort_by(|a, b| {
            // Unknowns sort last via the `~` sentinel.
            let ta = a.ty.map(|t| t.name()).unwrap_or("~");
            let tb = b.ty.map(|t| t.name()).unwrap_or("~");
            ta.cmp(tb)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        }),
    }
}

fn run_interactive_open(rows: &[TypedRow], reuse: bool, editor_override: Option<String>) {
    use inquire::Select;
    let options: Vec<String> = rows
        .iter()
        .map(|r| format!("{} → {}", r.name, r.path))
        .collect();

    let selected = match Select::new("Select a project to open:", options.clone())
        .with_page_size(15)
        .prompt()
    {
        Ok(s) => s,
        Err(_) => {
            log("Selection cancelled", LogType::Info);
            return;
        }
    };

    let idx = options.iter().position(|o| o == &selected).unwrap();
    let row = &rows[idx];
    let config = get_config();
    let editor = editor_override.as_deref().unwrap_or(&config.default_editor);
    open_and_exit(editor, &row.path, &row.name, reuse);
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
    let config = get_config();
    let editor = editor_override.as_deref().unwrap_or(&config.default_editor);

    // 1. Exact match in the registry
    if let Some(path) = projects.get(&project_name) {
        open_and_exit(editor, path, &project_name, reuse);
    }

    // 2. Path fallback — if the argument resolves to an existing directory,
    //    open it directly. Lets `vcode .`, `vcode ../foo`, `vcode ~/work/x` work.
    if let Some(resolved) = try_resolve_existing_dir(&project_name) {
        let display = path_basename(&resolved);
        open_and_exit(editor, &resolved.to_string_lossy(), &display, reuse);
    }

    // 3. Fuzzy match against project names (case-insensitive substring)
    let matches = fuzzy_match_projects(&projects, &project_name);
    match matches.len() {
        0 => {
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
        1 => {
            let (name, path) = &matches[0];
            log(
                &format!("→ Matched '{}'", name),
                LogType::Info,
            );
            open_and_exit(editor, path, name, reuse);
        }
        _ => {
            use inquire::Select;
            let options: Vec<String> = matches
                .iter()
                .map(|(n, p)| format!("{} → {}", n, p))
                .collect();
            match Select::new(
                &format!("Multiple matches for '{}'. Select one:", project_name),
                options,
            )
            .with_page_size(10)
            .prompt()
            {
                Ok(selected) => {
                    let name = selected.split(" → ").next().unwrap();
                    let path = matches
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, p)| p)
                        .unwrap();
                    open_and_exit(editor, path, name, reuse);
                }
                Err(_) => {
                    log("Selection cancelled", LogType::Info);
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Returns `(name, path)` pairs whose name contains `query` (case-insensitive),
/// sorted shortest-name-first so that closer-to-exact matches surface above
/// looser ones (e.g. `api` ranks above `api-service-backend`).
fn fuzzy_match_projects(
    projects: &std::collections::HashMap<String, String>,
    query: &str,
) -> Vec<(String, String)> {
    let q = query.to_lowercase();
    let mut out: Vec<(String, String)> = projects
        .iter()
        .filter(|(name, _)| name.to_lowercase().contains(&q))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    out.sort_by_key(|(n, _)| (n.len(), n.to_lowercase()));
    out
}

fn open_and_exit(editor: &str, path: &str, label: &str, reuse: bool) -> ! {
    match open_with_editor(editor, path, reuse) {
        Ok(()) => {
            log(
                &format!("Opening '{}' in {}", label, editor),
                LogType::Success,
            );
            std::process::exit(0);
        }
        Err(e) => {
            log(&format!("✗ Failed to open: {}", e), LogType::Error);
            std::process::exit(1);
        }
    }
}

pub fn handle_here(name: Option<String>, reuse: bool, editor_override: Option<String>) {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            log(&format!("✗ Could not determine current directory: {}", e), LogType::Error);
            std::process::exit(1);
        }
    };

    let project_name = name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| path_basename(&cwd));
    if project_name.is_empty() {
        log("✗ Could not infer project name from current directory", LogType::Error);
        std::process::exit(1);
    }

    let path_str = cwd.to_string_lossy().into_owned();
    if let Err(e) = set_project(&project_name, &path_str) {
        log(&format!("✗ Failed to register project: {}", e), LogType::Error);
        std::process::exit(1);
    }
    log(
        &format!("✓ Registered '{}' → {}", project_name, path_str),
        LogType::Success,
    );

    let config = get_config();
    let editor = editor_override.as_deref().unwrap_or(&config.default_editor);
    open_and_exit(editor, &path_str, &project_name, reuse);
}

pub fn handle_where(name: String) {
    let projects = get_projects();

    // Exact match → emit path.
    if let Some(path) = projects.get(&name) {
        println!("{}", path);
        return;
    }

    // Path fallback — print the canonical resolved path so scripts can use it
    // with `cd $(vcode where ../foo)`.
    if let Some(resolved) = try_resolve_existing_dir(&name) {
        println!("{}", resolved.display());
        return;
    }

    // Fuzzy match. For scripting safety, only emit when there's a single hit;
    // multiple matches go to stderr so command substitution captures nothing.
    let matches = fuzzy_match_projects(&projects, &name);
    match matches.len() {
        0 => {
            eprintln!("vcode: project '{}' not found", name);
            std::process::exit(1);
        }
        1 => {
            println!("{}", matches[0].1);
        }
        _ => {
            eprintln!("vcode: ambiguous match for '{}', candidates:", name);
            for (n, p) in &matches {
                eprintln!("  {} → {}", n, p);
            }
            std::process::exit(1);
        }
    }
}

pub fn handle_prune(yes: bool) {
    let mut projects = get_projects();
    let stale: Vec<(String, String)> = projects
        .iter()
        .filter(|(_, path)| {
            let p = Path::new(path);
            !p.exists() || !p.is_dir()
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    if stale.is_empty() {
        log("✓ No stale projects to prune", LogType::Success);
        return;
    }

    log(
        &format!(
            "Found {} stale project{}:",
            stale.len(),
            if stale.len() == 1 { "" } else { "s" }
        ),
        LogType::Info,
    );
    for (n, p) in &stale {
        println!("  - {} → {}", n, p);
    }

    if !yes {
        use inquire::Confirm;
        let confirm = Confirm::new("Remove these from the registry?")
            .with_default(false)
            .prompt();
        if !matches!(confirm, Ok(true)) {
            log("Cancelled", LogType::Info);
            return;
        }
    }

    for (n, _) in &stale {
        projects.remove(n);
    }
    match write_projects(&projects) {
        Ok(()) => log(
            &format!(
                "✓ Pruned {} project{}",
                stale.len(),
                if stale.len() == 1 { "" } else { "s" }
            ),
            LogType::Success,
        ),
        Err(e) => {
            log(&format!("✗ Failed to write registry: {}", e), LogType::Error);
            std::process::exit(1);
        }
    }
}

pub fn handle_update(name: String, path: String) {
    if !get_projects().contains_key(&name) {
        log(&format!("✗ Project '{}' not found", name), LogType::Error);
        std::process::exit(1);
    }

    match set_project_validated(&name, &path) {
        Ok(resolved) => log(
            &format!("✓ Updated '{}' → {}", name, resolved.display()),
            LogType::Success,
        ),
        Err(e) => {
            log(&format!("✗ {}", e), LogType::Error);
            std::process::exit(1);
        }
    }
}

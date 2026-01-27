use clap::{CommandFactory, Parser, Subcommand};
use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};
use std::path::PathBuf;
use vcode::{
    add_projects, config::{get_config, update_config, Config}, editor::open_with_editor, interactive_select_projects, log, scan_projects, FilterMode, LogType, project::{delete_project, get_projects, rename_project, reset_projects, resolve_path, set_project}, APP_NAME
};

/// A fast CLI project launcher for your favorite code editor
#[derive(Parser)]
#[command(
    name = APP_NAME,
    about = "Launch projects instantly by name",
    long_about = "vcode is a quick project launcher that opens your projects in your favorite editor by name, without navigating through directories.",
    version
)]
struct Cli {
    /// Project name to open
    project_name: Option<String>,

    /// Reuse existing editor window
    #[arg(short, long)]
    reuse: bool,

    /// Editor to use (overrides default)
    #[arg(short, long)]
    editor: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new project
    #[command(visible_alias = "a")]
    Add {
        /// Project name
        name: String,
        /// Project path
        path: String,
    },

    /// Remove a project
    #[command(visible_alias = "rm")]
    Remove {
        /// Project name
        name: String,
    },

    /// List all projects
    #[command(visible_alias = "ls")]
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Interactive mode - select a project to open
        #[arg(short, long)]
        interactive: bool,
    },

    /// Search projects by name or path
    #[command(visible_alias = "find")]
    Search {
        /// Search query
        query: String,
    },

    /// Rename a project
    #[command(visible_alias = "mv")]
    Rename {
        /// Current project name
        old_name: String,
        /// New project name
        new_name: String,
    },

    /// Scan directory for projects
    Scan {
        /// Directory to scan (defaults to configured projects_root)
        path: Option<String>,
        /// Depth to scan (default: 1)
        #[arg(short, long, default_value = "1")]
        depth: u32,
        /// Filter mode: auto (detect projects) or all (include all directories)
        #[arg(short, long, default_value = "auto")]
        filter: String,
        /// Skip interactive review and add all found projects
        #[arg(long)]
        no_review: bool,
    },

    /// View or update configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,
        /// Set projects root directory
        #[arg(long)]
        projects_root: Option<String>,
        /// Set default editor
        #[arg(long)]
        editor: Option<String>,
    },

    /// Clear all projects
    Clear {
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

fn print_table(projects: &std::collections::HashMap<String, String>) {
    if projects.is_empty() {
        log("No projects found. Add one with: vcode add <name> <path>", LogType::Info);
        return;
    }

    let mut sorted: Vec<_> = projects.iter().collect();
    sorted.sort_by_key(|(name, _)| name.to_lowercase());

    const PAGE_SIZE: usize = 20;
    let total_projects = sorted.len();
    let total_pages = (total_projects + PAGE_SIZE - 1) / PAGE_SIZE;

    if total_projects <= PAGE_SIZE {
        // Display all projects without pagination
        display_project_page(&sorted, 0, total_projects, 1, 1);
    } else {
        // Display with pagination
        let mut current_page = 0;

        loop {
            let start_idx = current_page * PAGE_SIZE;
            let end_idx = ((current_page + 1) * PAGE_SIZE).min(total_projects);

            display_project_page(
                &sorted[start_idx..end_idx],
                start_idx,
                total_projects,
                current_page + 1,
                total_pages,
            );

            if current_page < total_pages - 1 {
                use inquire::Select;
                let options = vec!["Next page", "Exit"];
                match Select::new("", options).without_help_message().prompt() {
                    Ok("Next page") => current_page += 1,
                    _ => break,
                }
            } else {
                break;
            }
        }
    }
}

fn display_project_page(
    projects: &[(&String, &String)],
    start_idx: usize,
    total: usize,
    current_page: usize,
    total_pages: usize,
) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#").fg(Color::Cyan),
            Cell::new("Name").fg(Color::Cyan),
            Cell::new("Path").fg(Color::Cyan),
        ]);

    for (idx, (name, path)) in projects.iter().enumerate() {
        table.add_row(vec![
            Cell::new(start_idx + idx + 1).fg(Color::DarkGrey),
            Cell::new(name).fg(Color::Green),
            Cell::new(path).fg(Color::White),
        ]);
    }

    println!("\n{}", table);

    if total_pages > 1 {
        println!(
            "\nShowing {} projects (Page {}/{}) | Total: {}\n",
            projects.len(),
            current_page,
            total_pages,
            total
        );
    } else {
        println!(
            "\nTotal: {} project{}\n",
            total,
            if total == 1 { "" } else { "s" }
        );
    }
}

fn main() {
    let cli = Cli::parse();
    let config = get_config();

    match cli.command {
        Some(cmd) => match cmd {
            Commands::Add { name, path } => {
                match set_project(&name, &resolve_path(&path).to_str().unwrap()) {
                    Ok(()) => log(&format!("✓ Added project '{}'", name), LogType::Success),
                    Err(_e) => log("✗ Failed to add project", LogType::Error),
                };
            }

            Commands::Remove { name } => match delete_project(&name) {
                Ok(()) => log(&format!("✓ Removed project '{}'", name), LogType::Success),
                Err(_) => log(&format!("✗ Project '{}' not found", name), LogType::Error),
            },

            Commands::List { json, interactive } => {
                let projects = get_projects();

                if json {
                    println!("{}", serde_json::to_string_pretty(&projects).unwrap());
                    return;
                }

                if interactive {
                    if projects.is_empty() {
                        log("No projects found. Add one with: vcode add <name> <path>", LogType::Info);
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
                            let editor = cli.editor.as_deref().unwrap_or(&config.default_editor);

                            if let Some(path) = projects.get(project_name) {
                                match open_with_editor(editor, path, cli.reuse) {
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

            Commands::Search { query } => {
                let projects = get_projects();
                let query_lower = query.to_lowercase();
                let filtered: std::collections::HashMap<_, _> = projects
                    .iter()
                    .filter(|(name, path)| {
                        name.to_lowercase().contains(&query_lower)
                            || path.to_lowercase().contains(&query_lower)
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                if filtered.is_empty() {
                    log(&format!("No projects found matching '{}'", query), LogType::Info);
                } else {
                    log(&format!("Projects matching '{}':", query), LogType::Info);
                    print_table(&filtered);
                }
            }

            Commands::Rename { old_name, new_name } => {
                match rename_project(&old_name, &new_name) {
                    Ok(()) => log(
                        &format!("✓ Renamed '{}' → '{}'", old_name, new_name),
                        LogType::Success,
                    ),
                    Err(e) => log(&format!("✗ {}", e), LogType::Error),
                }
            }

            Commands::Scan {
                path,
                depth,
                filter,
                no_review,
            } => {
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
                            &format!("✓ Found {} project{}", project_count, if project_count == 1 { "" } else { "s" }),
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

            Commands::Config {
                show,
                projects_root,
                editor,
            } => {
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

            Commands::Clear { yes } => {
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
        },
        None => match cli.project_name {
            Some(project_name) => {
                let projects = get_projects();
                let path = projects.get(&project_name);

                let editor = cli.editor.as_deref().unwrap_or(&config.default_editor);

                match path {
                    None => {
                        log(&format!("✗ Project '{}' not found", project_name), LogType::Error);
                        log("\nTip: Use 'vcode list' to see all projects or 'vcode add' to add a new one", LogType::Info);
                        std::process::exit(1);
                    }
                    Some(path) => match open_with_editor(editor, path, cli.reuse) {
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
            None => {
                log("vcode - Quick Project Launcher", LogType::Info);
                println!();
                let mut cmd = Cli::command();
                let _ = cmd.print_help();
                std::process::exit(1);
            }
        },
    }
}

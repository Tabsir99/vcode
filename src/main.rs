use clap::{CommandFactory, Parser, Subcommand};
use vcode::{APP_NAME, LogType, commands, log};

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => match cmd {
            Commands::Add { name, path } => commands::handle_add(name, path),
            Commands::Remove { name } => commands::handle_remove(name),
            Commands::List { json, interactive } => {
                commands::handle_list(json, interactive, cli.reuse, cli.editor)
            }
            Commands::Search { query } => commands::handle_search(query),
            Commands::Rename { old_name, new_name } => commands::handle_rename(old_name, new_name),
            Commands::Scan {
                path,
                depth,
                filter,
                no_review,
            } => commands::handle_scan(path, depth, filter, no_review),
            Commands::Config {
                show,
                projects_root,
                editor,
            } => commands::handle_config(show, projects_root, editor),
            Commands::Clear { yes } => commands::handle_clear(yes),
        },
        None => match cli.project_name {
            Some(project_name) => {
                commands::handle_open_project(project_name, cli.reuse, cli.editor)
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

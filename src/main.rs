use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use vcode::{APP_NAME, LogType, commands, commands::ConfigAction, commands::SortKey, log};

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

    /// Copy `cd <path>` to the clipboard instead of the default action
    /// (works with `vcode <project>`, `vcode where`, and `vcode find`).
    #[arg(long, global = true)]
    cd: bool,

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
        /// Project path (optional when using --find)
        path: Option<String>,
        /// Search filesystem for directory with this name and add it
        #[arg(long, short = 'f')]
        find: bool,
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
        /// Sort projects by this key
        #[arg(short, long, value_enum, default_value_t = SortKey::Name)]
        sort: SortKey,
        /// Filter by project type (e.g. rust, javascript, python, go)
        #[arg(short = 'F', long)]
        filter: Option<String>,
    },

    /// Search projects by name or path
    #[command(visible_alias = "find")]
    Search {
        /// Search query
        query: String,
        /// Search the actual filesystem (under $HOME) for directories whose
        /// name contains the query, then add the selected ones as projects.
        /// Use this when you know a project exists on disk but `vcode scan`
        /// didn't pick it up (e.g. it's outside your projects root).
        #[arg(short = 'f', long)]
        fs: bool,
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

    /// Manage configuration
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },

    /// Clear all projects
    Clear {
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Register the current directory as a project and open it
    Here {
        /// Project name (defaults to current directory basename)
        name: Option<String>,
    },

    /// Print the path of a project (for shell scripting, e.g. `cd $(vcode where api)`)
    Where {
        /// Project name (supports fuzzy match)
        name: String,
    },

    /// Remove projects whose paths no longer exist on disk
    Prune {
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Change the path of an existing project
    Update {
        /// Project name
        name: String,
        /// New path
        path: String,
    },

    /// Generate shell completion script (bash, zsh, fish, powershell, elvish)
    Completions {
        /// Target shell
        shell: Shell,
    },
}


fn main() {
    // Hidden re-exec used by `--cd` on Linux to keep the clipboard alive
    // after the user's invocation returns. Intercepted here before clap
    // parsing so it never shows up in `--help`.
    #[cfg(target_os = "linux")]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.len() >= 3 && args[1] == vcode::core::clipboard::DAEMON_SUBCOMMAND {
            vcode::core::clipboard::run_daemon(&args[2]);
        }
    }

    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => match cmd {
            Commands::Add { name, path, find } => commands::handle_add(name, path, find),
            Commands::Remove { name } => commands::handle_remove(name),
            Commands::List {
                json,
                interactive,
                sort,
                filter,
            } => commands::handle_list(json, interactive, cli.reuse, cli.editor, sort, filter),
            Commands::Search { query, fs } => commands::handle_search(query, fs, cli.cd),
            Commands::Rename { old_name, new_name } => commands::handle_rename(old_name, new_name),
            Commands::Scan {
                path,
                depth,
                filter,
                no_review,
            } => commands::handle_scan(path, depth, filter, no_review),
            Commands::Config { action } => commands::handle_config(action),
            Commands::Clear { yes } => commands::handle_clear(yes),
            Commands::Here { name } => commands::handle_here(name, cli.reuse, cli.editor),
            Commands::Where { name } => commands::handle_where(name, cli.cd),
            Commands::Prune { yes } => commands::handle_prune(yes),
            Commands::Update { name, path } => commands::handle_update(name, path),
            Commands::Completions { shell } => {
                let mut cmd = Cli::command();
                clap_complete::generate(shell, &mut cmd, APP_NAME, &mut std::io::stdout());
            }
        },
        None => match cli.project_name {
            Some(project_name) => {
                commands::handle_open_project(project_name, cli.reuse, cli.editor, cli.cd)
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

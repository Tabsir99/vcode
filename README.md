# vcode

> A fast, modern CLI project launcher for your favorite code editor

**vcode** lets you open projects instantly by name, without navigating through directories. Just type `vcode myproject` and you're in!

## Installation

```bash
# From crates.io
cargo install vcode

# Or from source
git clone https://github.com/Tabsir99/vcode
cd vcode && cargo install --path .
```

## Quick Start

```bash
vcode myproject           # Open project in default editor
vcode myproject -e nvim   # Open with specific editor
vcode myproject -r        # Reuse existing window
```

On first run, vcode will prompt you for your projects directory and default editor.

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `vcode <name>` | - | Open project by name |
| `vcode add <name> <path>` | `a` | Add project manually |
| `vcode remove <name>` | `rm` | Remove a project |
| `vcode list` | `ls` | List all projects |
| `vcode search <query>` | `find` | Search by name or path |
| `vcode rename <old> <new>` | `mv` | Rename a project |
| `vcode scan [path]` | - | Auto-discover projects |
| `vcode config` | `cfg` | Manage configuration |
| `vcode clear` | - | Remove all projects |

### Scan Options

```bash
vcode scan                    # Scan configured projects root
vcode scan ~/work --depth 3   # Scan custom path, 3 levels deep
vcode scan --filter all       # Include all directories
vcode scan --no-review        # Skip interactive selection
```

### Config Subcommands

```bash
vcode config                  # Show current config
vcode config show             # Show current config (explicit)
vcode config set editor nvim  # Set default editor
vcode config set projects-root ~/dev
vcode config editors          # List all registered editors
vcode config add              # Add custom editor (interactive)
vcode config remove helix     # Remove an editor
vcode config edit             # Interactive configuration wizard
vcode config reset            # Reset to defaults
```

### List Options

```bash
vcode list --json        # Output as JSON
vcode list -i            # Select and open interactively
```

## Project Detection

When scanning, vcode detects projects by their markers:

| Type | Markers |
|------|---------|
| Rust | `Cargo.toml` |
| JavaScript/TypeScript | `package.json`, `tsconfig.json`, `deno.json` |
| Python | `requirements.txt`, `pyproject.toml`, `Pipfile` |
| Go | `go.mod` |
| Java | `pom.xml`, `build.gradle` |
| C# | `.csproj`, `.sln` |
| C/C++ | `CMakeLists.txt`, `Makefile` |
| Ruby | `Gemfile` |
| PHP | `composer.json` |
| Git | `.git` directory |

## Data Storage

| File | Location |
|------|----------|
| Configuration | `~/.config/vcode/config.json` |
| Projects | `~/.local/share/vcode/projects.json` |

### Config Structure

```json
{
  "projects_root": "/home/user/projects",
  "default_editor": "cursor",
  "editors": {
    "cursor": { "command": "cursor", "args": ["--no-sandbox"] },
    "nvim": { "command": "nvim", "args": [] }
  }
}
```

### Projects Structure

```json
{
  "api-service": "/home/user/projects/api-service",
  "frontend": "/home/user/projects/frontend"
}
```

## Project Structure

```
src/
├── main.rs          # CLI entry point and argument parsing
├── lib.rs           # Library root with module exports
├── commands.rs      # Command handlers (add, remove, list, etc.)
├── core/
│   ├── config.rs    # Configuration management
│   ├── project.rs   # Project CRUD operations
│   └── editor.rs    # Editor launching logic
├── scanner/
│   ├── scanner.rs   # Directory traversal and project discovery
│   └── detector.rs  # Project type detection by markers
└── ui/
    ├── logger.rs    # Colored console output
    └── display.rs   # Table formatting with pagination
```

## Examples

```bash
# Daily workflow
vcode api                     # Open your API project
vcode search backend          # Find backend-related projects
vcode list -i                 # Browse and open interactively

# Initial setup
vcode scan ~/projects --depth 2   # Discover all projects
vcode add mobile ~/work/mobile    # Add project manually

# Configuration
vcode config add                  # Register custom editor (e.g., helix, zed)
vcode config set editor nvim      # Set as default
vcode config editors              # See all available editors
```

## License

MIT

---

Made for developers who value their time

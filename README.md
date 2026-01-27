# vcode

> A fast, modern CLI project launcher for your favorite code editor

**vcode** lets you open projects instantly by name, without navigating through directories. Just type `vcode myproject` and you're in!

## Features

- **Quick Launch** - Open projects by name in seconds
- **Smart Search** - Find projects by name or path
- **Project Detection** - Automatically detects Rust, JavaScript, Python, Go, Java, C++, Ruby, PHP, and more
- **Interactive Mode** - Browse and select projects with modern TUI
- **Beautiful Tables** - Clean, modern output with pagination
- **Multiple Editors** - Support for VS Code, Cursor, VSCodium
- **Smart Scan** - Intelligently discovers projects with filtering options
- **Fast & Lightweight** - Minimal dependencies, instant startup

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/Tabsir99/vcode
cd vcode
cargo build --release
```

## Quick Start

### First Time Setup

On first run, vcode will prompt you for:
- **Projects Root** - Directory where your projects live (e.g., `~/projects`)
- **Default Editor** - Your preferred editor (`code`, `cursor`, etc.)

### Basic Usage

```bash
vcode myproject

vcode myproject -e cursor

vcode myproject -r
```

## Commands

### Project Management

```bash
vcode add myproject ~/path/to/project
vcode a myproject ~/path/to/project

vcode remove myproject
vcode rm myproject

vcode rename old-name new-name
vcode mv old-name new-name
```

### Discovery & Search

```bash
vcode list
vcode ls

vcode list --interactive
vcode ls -i

vcode search react
vcode find react

vcode scan
vcode scan ~/my-projects
vcode scan ~/my-projects --depth 3

vcode scan --filter auto
vcode scan --filter all

vcode scan --no-review
```

### Configuration

```bash
vcode config
vcode config --show

vcode config --editor cursor
vcode config --projects-root ~/projects

vcode clear
vcode clear --yes
```

### Output Formats

```bash
vcode list --json
```

## Smart Project Detection

vcode can automatically detect projects by their common markers:

| Language/Type | Markers |
|---------------|---------|
| Rust | `Cargo.toml` |
| JavaScript | `package.json` |
| TypeScript | `tsconfig.json`, `deno.json` |
| Python | `requirements.txt`, `setup.py`, `pyproject.toml`, `Pipfile` |
| Go | `go.mod` |
| Java | `pom.xml`, `build.gradle` |
| C# | `.csproj`, `.sln` |
| C/C++ | `CMakeLists.txt`, `Makefile` |
| Ruby | `Gemfile` |
| PHP | `composer.json` |
| Git Repo | `.git` directory |

When scanning with `--filter auto`, only directories with these markers are included.

## Interactive Features

### Interactive Scan
After scanning, review and select which projects to add:
- Use **Space** to toggle selection
- Use **Arrow keys** to navigate
- Press **Enter** to confirm
- All projects selected by default

### Interactive List
Browse your projects and open them directly:
```bash
vcode list --interactive
```
- Search through projects with fuzzy matching
- Select with arrow keys
- Opens immediately in your configured editor

## Examples

### Daily Workflow

```bash
vcode work

vcode search backend
vcode api-service

vcode config
```

### Bulk Setup

```bash
vcode scan ~/projects --depth 2

vcode scan ~/projects --depth 2 --no-review

vcode scan ~/projects --filter all

vcode search "react"
```

### Project Organization

```bash
vcode add api ~/work/api-service
vcode add frontend ~/work/web-app
vcode add mobile ~/work/mobile-app

vcode list

vcode rename api backend-api
```

## Output Examples

### List Command
```
┌────┬──────────────┬─────────────────────────────────────┐
│ #  ┆ Name         ┆ Path                                │
╞════╪══════════════╪═════════════════════════════════════╡
│ 1  ┆ api-service  ┆ /home/user/projects/api-service     │
├╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ 2  ┆ frontend     ┆ /home/user/projects/frontend        │
├╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ 3  ┆ mobile       ┆ /home/user/projects/mobile          │
└────┴──────────────┴─────────────────────────────────────┘

Total: 3 projects
```

### Config Command
```
Configuration:

  Editor:        cursor
  Projects Root: /home/user/projects
```

### Success Messages
```
Added project 'myproject'
Removed project 'oldproject'
Renamed 'api' → 'backend-api'
Opening 'myproject' in cursor
```

## Project Structure

```
src/
├── lib.rs
├── main.rs
├── bulk.rs
├── detector.rs
├── config.rs
├── editor.rs
├── logger.rs
└── project.rs
```

## Configuration Files

- **Config**: `~/.config/vcode/config.json`
- **Projects**: `~/.local/share/vcode/projects.json`

## Command Aliases

For convenience, most commands have short aliases:

| Command | Alias |
|---------|-------|
| `add`   | `a`   |
| `remove`| `rm`  |
| `list`  | `ls`  |
| `search`| `find`|
| `rename`| `mv`  |

## Tips & Tricks

1. **Interactive Mode**: Use `vcode list -i` for a quick way to browse and open projects
2. **Smart Scanning**: Start with `--depth 2` or `--depth 3` to find projects nested in category folders
3. **Depth Explained**: The scan checks **all levels up to** the specified depth, finding projects wherever they are
4. **Filter Modes**: Use `--filter auto` (default) to only add recognized projects, or `--filter all` to include everything
5. **Quick Search**: Use `vcode search` to quickly filter large project lists
6. **JSON Export**: Use `vcode list --json` for integration with other tools
7. **Pagination**: When you have 20+ projects, list automatically paginates for readability

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

### Contributing

Contributions are welcome! The modular structure makes it easy to add new features.

## Design Philosophy

- **Simple First**: Most commands work without flags
- **Natural Language**: Commands read like sentences
- **Beautiful Output**: Clean, modern formatting
- **Fast**: Optimized for instant feedback
- **Scalable**: Modular architecture for easy extensions

## License

MIT

---

Made for developers who value their time

use crate::APP_NAME;
use dirs;
use inquire::{Select, Text};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EditorConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub reuse_flag: Option<String>,
}

impl EditorConfig {
    pub fn new(command: String) -> Self {
        Self {
            command,
            args: Vec::new(),
            reuse_flag: Some("-r".to_string()),
        }
    }

    pub fn vscode_like(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: vec!["--no-sandbox".to_string()],
            reuse_flag: Some("-r".to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub projects_root: String,
    pub default_editor: String,
    #[serde(default)]
    pub editors: HashMap<String, EditorConfig>,
}

impl Config {
    pub fn new(projects_root: String, default_editor: String) -> Self {
        Self {
            projects_root,
            default_editor,
            editors: default_editors(),
        }
    }

    pub fn get_editor(&self, name: &str) -> Option<&EditorConfig> {
        self.editors.get(name)
    }

    pub fn add_editor(&mut self, name: String, config: EditorConfig) {
        self.editors.insert(name, config);
    }

    pub fn remove_editor(&mut self, name: &str) -> bool {
        self.editors.remove(name).is_some()
    }
}

fn default_editors() -> HashMap<String, EditorConfig> {
    let mut editors = HashMap::new();
    editors.insert("code".to_string(), EditorConfig::vscode_like("code"));
    editors.insert("cursor".to_string(), EditorConfig::vscode_like("cursor"));
    editors.insert("vscodium".to_string(), EditorConfig::vscode_like("vscodium"));
    editors.insert("zed".to_string(), EditorConfig::new("zed".to_string()));
    editors.insert("nvim".to_string(), EditorConfig::new("nvim".to_string()));
    editors.insert("vim".to_string(), EditorConfig::new("vim".to_string()));
    editors.insert("emacs".to_string(), EditorConfig::new("emacs".to_string()));
    editors.insert("sublime".to_string(), EditorConfig::new("subl".to_string()));
    editors
}

pub fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join(APP_NAME)
        .join("config.json")
}

pub fn init_config() -> Config {
    let config_dir = dirs::config_dir()
        .expect("Could not find config directory")
        .join(APP_NAME);
    let config_path = config_dir.join("config.json");

    create_dir_all(&config_dir).expect("Failed to create config directory");

    println!("First time setup!");

    let projects_root =
        Text::new("Provide a path to the directory that contains all your projects:")
            .with_default(&dirs::home_dir().unwrap().join("projects").to_string_lossy())
            .prompt()
            .expect("Failed to get projects root");

    let editors = default_editors();
    let editor_names: Vec<&str> = editors.keys().map(|s| s.as_str()).collect();

    let default_editor = Select::new("Choose your default editor:", editor_names)
        .prompt()
        .expect("Failed to get editor choice");

    let default_config = Config {
        projects_root,
        default_editor: default_editor.to_string(),
        editors,
    };

    let config_json =
        serde_json::to_string_pretty(&default_config).expect("Failed to serialize config");
    write(&config_path, config_json).expect("Failed to write config file");

    default_config
}

pub fn get_config() -> Config {
    let config_path = get_config_path();

    match read_to_string(&config_path) {
        Ok(config_str) => serde_json::from_str(&config_str).expect("Failed to parse config json"),
        Err(_) => init_config(),
    }
}

pub fn update_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let config_json = serde_json::to_string_pretty(config)?;
    write(config_path, config_json)?;
    Ok(())
}

pub fn reset_config() -> Config {
    let config_path = get_config_path();

    // Remove existing config
    let _ = std::fs::remove_file(&config_path);

    // Run init again
    init_config()
}

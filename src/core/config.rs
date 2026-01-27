use crate::APP_NAME;
use dirs;
use inquire::{Select, Text};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Config {
    pub projects_root: String,
    pub default_editor: String,
}

impl Config {
    pub fn new(projects_root: String, default_editor: String) -> Self {
        Self {
            projects_root,
            default_editor,
        }
    }
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

    let editors = vec!["code", "cursor"];
    let default_editor = Select::new("Choose your default editor:", editors)
        .prompt()
        .expect("Failed to get editor choice");

    let default_config = Config {
        projects_root,
        default_editor: default_editor.to_string(),
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

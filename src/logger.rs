use colored::Colorize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogType {
    Info,
    Error,
    Success,
    Warning,
    Normal,
}

pub fn log(msg: &str, log_type: LogType) {
    match log_type {
        LogType::Info => println!("{}", msg.bright_blue().bold()),
        LogType::Error => println!("{}", msg.red().bold()),
        LogType::Success => println!("{}", msg.green().bold()),
        LogType::Normal => println!("{}", msg.bold().truecolor(30, 30, 30)),
        LogType::Warning => println!("{}", msg.yellow().bold()),
    }
}

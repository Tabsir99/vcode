//! User interface components
//!
//! This module provides UI-related functionality:
//! - Logging and colored output (logger.rs)
//! - Table display for projects (display.rs)

pub mod display;
pub mod logger;

// Re-export commonly used items
pub use display::print_table;
pub use logger::{LogType, log};

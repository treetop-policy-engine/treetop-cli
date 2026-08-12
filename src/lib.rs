//! Standalone command-line client and interactive REPL for Treetop servers.

#![forbid(unsafe_code)]

pub mod app;
pub mod cli_config;
pub mod completion;
pub mod matrix;
pub mod models;
pub mod paths;
pub mod repl;
pub mod style;

pub use app::run;
pub use cli_config::CliConfig;
pub use completion::*;
pub use matrix::*;
pub use models::*;

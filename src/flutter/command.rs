use std::path::{Path, PathBuf};

use crate::common::ansi::{gray, red};

/// Encapsulates the resolved Flutter executable and prefix arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterCommand {
    pub executable: String,
    pub prefix: Vec<String>,
}

impl FlutterCommand {
    pub fn new(executable: impl Into<String>, prefix: Vec<String>) -> Self {
        Self {
            executable: executable.into(),
            prefix,
        }
    }

    /// Combines prefix arguments with the provided arguments.
    pub fn with_args(&self, args: &[String]) -> Vec<String> {
        let mut combined = self.prefix.clone();
        combined.extend_from_slice(args);
        combined
    }

    /// Formats the command and arguments for terminal display.
    pub fn describe(&self, command_args: &[String]) -> String {
        if command_args.is_empty() {
            return self.executable.clone();
        }
        format!("{} {}", self.executable, command_args.join(" "))
    }

    /// Runs Flutter command with standard input/output streaming directly to the terminal.
    pub async fn run_passthrough(&self, arguments: &[String], verbose: bool) -> i32 {
        let command_args = self.with_args(arguments);
        if verbose {
            println!(
                "{}",
                gray(&format!("Running: {}", self.describe(&command_args)))
            );
        }

        let mut child = match tokio::process::Command::new(&self.executable)
            .args(&command_args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                eprintln!(
                    "{}",
                    red(&format!("Failed to spawn Flutter command: {error}"))
                );
                return 1;
            }
        };

        match child.wait().await {
            Ok(status) => status.code().unwrap_or(0),
            Err(error) => {
                eprintln!("{}", red(&format!("Error waiting for command: {error}")));
                1
            }
        }
    }
}

/// Detects whether to execute Flutter via FVM or global installation.
pub fn resolve_flutter_command(start_directory: Option<&Path>) -> FlutterCommand {
    let mut current_dir = match start_directory {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };

    loop {
        let fvm_dir = current_dir.join(".fvm");
        let config_file = current_dir.join("fvm_config.json");
        let nested_config = fvm_dir.join("fvm_config.json");

        if fvm_dir.is_dir() || config_file.is_file() || nested_config.is_file() {
            return FlutterCommand::new("fvm", vec!["flutter".to_string()]);
        }

        let parent = match current_dir.parent() {
            Some(parent) => parent.to_path_buf(),
            None => break,
        };

        if parent == current_dir {
            break;
        }

        current_dir = parent;
    }

    FlutterCommand::new("flutter", Vec::new())
}

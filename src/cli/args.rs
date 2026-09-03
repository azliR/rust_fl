use std::fmt;

use crate::common::constants::{is_supported_platform, supported_platforms_string};

/// Error thrown when invalid command line arguments or options are provided.
#[derive(Debug, PartialEq, Eq)]
pub struct UsageError {
    pub message: String,
}

impl UsageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for UsageError {}

/// Holds parsed top-level CLI flags and command routing data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArgs {
    pub show_help: bool,
    pub show_version: bool,
    pub verbose: bool,
    pub command: Option<String>,
    pub command_args: Vec<String>,
}

/// Arguments extracted specifically for `fl run`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommandArgs {
    pub cleaned_args: Vec<String>,
    pub platform_override: Option<String>,
    pub force_device_refresh: bool,
    pub auto_yes: bool,
}

/// Parses top-level CLI arguments before the primary command.
pub fn parse_arguments(arguments: &[String]) -> Result<ParsedArgs, UsageError> {
    let mut show_help = false;
    let mut show_version = false;
    let mut verbose = false;
    let mut command = None;
    let mut command_args = Vec::new();

    let mut index = 0;

    while index < arguments.len() {
        let current = &arguments[index];

        if current == "--" {
            index += 1;
            break;
        }

        if current == "--help" || current == "-h" {
            show_help = true;
            index += 1;
            continue;
        }

        if current == "--version" {
            show_version = true;
            index += 1;
            continue;
        }

        if current == "--verbose" || current == "-v" {
            verbose = true;
            index += 1;
            continue;
        }

        if current.starts_with('-') {
            return Err(UsageError::new(format!(
                "Unknown global option: {current}\n\
                 Global options must come before the command.\n\
                 Use \"fl <command> --help\" to see command-specific options."
            )));
        }

        command = Some(current.clone());
        index += 1;
        break;
    }

    while index < arguments.len() {
        command_args.push(arguments[index].clone());
        index += 1;
    }

    Ok(ParsedArgs {
        show_help,
        show_version,
        verbose,
        command,
        command_args,
    })
}

/// Extracts arguments specific to `fl run` and separates forwardable Flutter arguments.
pub fn extract_run_command_args(arguments: &[String]) -> Result<RunCommandArgs, UsageError> {
    let mut cleaned_args = Vec::new();
    let mut platform_override = None;
    let mut saw_double_dash = false;
    let mut force_device_refresh = false;
    let mut auto_yes = false;

    let mut index = 0;
    while index < arguments.len() {
        let current = &arguments[index];

        if current == "--" {
            saw_double_dash = true;
            cleaned_args.push(current.clone());
            index += 1;
            continue;
        }

        if !saw_double_dash && current == "--force-device-refresh" {
            force_device_refresh = true;
            index += 1;
            continue;
        }

        if !saw_double_dash && (current == "-y" || current == "--yes") {
            auto_yes = true;
            index += 1;
            continue;
        }

        if !saw_double_dash && current == "--platform" {
            if platform_override.is_some() {
                return Err(UsageError::new(
                    "Multiple --platform arguments are not allowed.",
                ));
            }
            if index + 1 >= arguments.len() {
                return Err(UsageError::new(
                    "Expected a platform name after --platform.",
                ));
            }
            index += 1;
            platform_override = Some(normalize_platform_value(&arguments[index])?);
            index += 1;
            continue;
        }

        if !saw_double_dash && current.starts_with("--platform=") {
            if platform_override.is_some() {
                return Err(UsageError::new(
                    "Multiple --platform arguments are not allowed.",
                ));
            }
            let value = &current["--platform=".len()..];
            platform_override = Some(normalize_platform_value(value)?);
            index += 1;
            continue;
        }

        cleaned_args.push(current.clone());
        index += 1;
    }

    Ok(RunCommandArgs {
        cleaned_args,
        platform_override,
        force_device_refresh,
        auto_yes,
    })
}

/// Validates and normalizes user input for platform selection.
pub fn normalize_platform_value(raw_input: &str) -> Result<String, UsageError> {
    let normalized = raw_input.trim().to_lowercase();
    if normalized.is_empty() {
        return Err(UsageError::new(
            "Expected a platform name after --platform.",
        ));
    }
    if !is_supported_platform(&normalized) {
        return Err(UsageError::new(format!(
            "Unknown platform: {raw_input}.\nSupported platforms: {}.",
            supported_platforms_string()
        )));
    }
    Ok(normalized)
}

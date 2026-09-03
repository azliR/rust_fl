use std::fs;
use std::path::Path;

use regex::Regex;

use crate::common::ansi::{gray, green, red};

/// Represents a single dependency entry and its nested YAML lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub lines: Vec<String>,
}

impl Dependency {
    pub fn new(name: impl Into<String>, lines: Vec<String>) -> Self {
        Self {
            name: name.into(),
            lines,
        }
    }
}

/// Sorts `dependencies` and `dev_dependencies` entries alphabetically in `pubspec.yaml`.
pub fn sort_pubspec(verbose: bool, create_backup: bool) -> Result<(), String> {
    let pubspec_path = Path::new("pubspec.yaml");

    if !pubspec_path.is_file() {
        eprintln!(
            "{}",
            red("Error: pubspec.yaml not found in current directory")
        );
        return Err("pubspec.yaml not found".to_string());
    }

    if verbose {
        println!("{}", gray("Reading pubspec.yaml..."));
    }

    let content = match fs::read_to_string(pubspec_path) {
        Ok(content) => content,
        Err(error) => {
            let msg = format!("Error reading pubspec.yaml: {error}");
            eprintln!("{}", red(&msg));
            return Err(msg);
        }
    };

    let sorted_content = match sort_pubspec_content(&content) {
        Ok(sorted) => sorted,
        Err(error) => {
            let msg = format!("Error sorting pubspec.yaml: {error}");
            eprintln!("{}", red(&msg));
            return Err(msg);
        }
    };

    if create_backup {
        let backup_path = Path::new("pubspec.yaml.backup");
        if let Err(error) = fs::write(backup_path, &content) {
            let msg = format!("Error creating backup: {error}");
            eprintln!("{}", red(&msg));
            return Err(msg);
        }
        if verbose {
            println!("{}", gray("Created backup: pubspec.yaml.backup"));
        }
    }

    if let Err(error) = fs::write(pubspec_path, sorted_content) {
        let msg = format!("Error writing pubspec.yaml: {error}");
        eprintln!("{}", red(&msg));
        return Err(msg);
    }

    println!("{}", green("✓ Successfully sorted pubspec.yaml"));
    if create_backup {
        println!("{}", gray("  Backup saved to: pubspec.yaml.backup"));
    }

    Ok(())
}

/// Sorts dependencies inside raw pubspec content string.
pub fn sort_pubspec_content(content: &str) -> Result<String, String> {
    let lines: Vec<&str> = content.split('\n').collect();
    let mut result = Vec::new();
    let mut in_dependencies = false;
    let mut in_dev_dependencies = false;
    let mut current_section: Vec<String> = Vec::new();
    let mut section_indent = String::new();
    let mut trailing_empty_lines: Vec<String> = Vec::new();

    let indent_regex = Regex::new(r"^(\s+)").map_err(|e| e.to_string())?;

    for line in lines {
        let trimmed = line.trim();

        if trimmed == "dependencies:" {
            if !current_section.is_empty() {
                result.extend(sort_dependency_section(&current_section, &section_indent)?);
                current_section.clear();
            }
            result.append(&mut trailing_empty_lines);

            in_dependencies = true;
            in_dev_dependencies = false;
            result.push(line.to_string());
            continue;
        }

        if trimmed == "dev_dependencies:" {
            if !current_section.is_empty() {
                result.extend(sort_dependency_section(&current_section, &section_indent)?);
                current_section.clear();
            }
            result.append(&mut trailing_empty_lines);

            in_dev_dependencies = true;
            in_dependencies = false;
            result.push(line.to_string());
            continue;
        }

        if (in_dependencies || in_dev_dependencies)
            && !line.is_empty()
            && !line.starts_with(' ')
            && !line.starts_with('\t')
        {
            if !current_section.is_empty() {
                result.extend(sort_dependency_section(&current_section, &section_indent)?);
                current_section.clear();
            }
            result.append(&mut trailing_empty_lines);

            in_dependencies = false;
            in_dev_dependencies = false;
            result.push(line.to_string());
            continue;
        }

        if in_dependencies || in_dev_dependencies {
            if !trimmed.is_empty() {
                if !trailing_empty_lines.is_empty() {
                    current_section.append(&mut trailing_empty_lines);
                }

                if current_section.is_empty()
                    && line.starts_with(' ')
                    && let Some(captures) = indent_regex.captures(line)
                    && let Some(matched) = captures.get(1)
                {
                    section_indent = matched.as_str().to_string();
                }
                current_section.push(line.to_string());
            } else {
                trailing_empty_lines.push(line.to_string());
            }
        } else {
            result.push(line.to_string());
        }
    }

    if !current_section.is_empty() {
        result.extend(sort_dependency_section(&current_section, &section_indent)?);
    }
    result.extend(trailing_empty_lines);

    Ok(result.join("\n"))
}

/// Sorts a dependency section and groups multi-line entries together.
pub fn sort_dependency_section(section: &[String], indent: &str) -> Result<Vec<String>, String> {
    if section.is_empty() {
        return Ok(Vec::new());
    }

    let indent_regex = Regex::new(r"^(\s+)").map_err(|e| e.to_string())?;
    let name_regex = Regex::new(r"^\s*([^:]+):").map_err(|e| e.to_string())?;

    let mut dependencies: Vec<Dependency> = Vec::new();
    let mut index = 0;

    while index < section.len() {
        let line = &section[index];

        if line.trim().is_empty() {
            index += 1;
            continue;
        }

        if line.starts_with(indent) && line.trim().contains(':') {
            let mut dependency_lines = vec![line.clone()];
            let current_indent_length = indent.len();
            let mut next_index = index + 1;

            while next_index < section.len() {
                let sub_line = &section[next_index];
                if sub_line.trim().is_empty() {
                    next_index += 1;
                    break;
                }

                let sub_indent_length = indent_regex
                    .captures(sub_line)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().len());

                if let Some(length) = sub_indent_length
                    && length > current_indent_length
                {
                    dependency_lines.push(sub_line.clone());
                    next_index += 1;
                    continue;
                }
                break;
            }

            if let Some(captures) = name_regex.captures(line)
                && let Some(name_match) = captures.get(1)
            {
                dependencies.push(Dependency::new(
                    name_match.as_str().trim(),
                    dependency_lines,
                ));
            }

            index = next_index;
            continue;
        }

        index += 1;
    }

    dependencies.sort_by_key(|a| a.name.to_lowercase());

    let mut result = Vec::new();
    for dependency in dependencies {
        result.extend(dependency.lines);
    }

    Ok(result)
}

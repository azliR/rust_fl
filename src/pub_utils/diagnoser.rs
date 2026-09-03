use std::fs;
use std::path::Path;

use crate::common::ansi::{cyan, format_timestamp, gray, green, red, yellow};
use crate::flutter::command::{FlutterCommand, resolve_flutter_command};
use crate::pub_utils::solver_parser::{PubDiagnosisReport, parse_pub_solver_error};

/// Runs dependency diagnosis or parses a provided solver error log.
pub async fn run_dependency_diagnosis(
    flutter_command: Option<&FlutterCommand>,
    run_upgrade: bool,
    input_file_path: Option<&str>,
    verbose: bool,
) -> i32 {
    if let Some(path_str) = input_file_path {
        let input_path = Path::new(path_str);
        if !input_path.is_file() {
            eprintln!(
                "{}",
                red(&format!("Error: Input file not found at {path_str}"))
            );
            return 1;
        }

        let content = match fs::read_to_string(input_path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!("{}", red(&format!("Error reading input file: {error}")));
                return 1;
            }
        };

        let report = parse_pub_solver_error(&content);
        display_diagnosis_report(&report, None);
        return 0;
    }

    let pubspec_path = Path::new("pubspec.yaml");
    if !pubspec_path.is_file() {
        eprintln!(
            "{}",
            red("Error: pubspec.yaml not found in current directory")
        );
        return 1;
    }

    let fallback_command = resolve_flutter_command(None);
    let resolved_command = flutter_command.unwrap_or(&fallback_command);

    let dry_run_args = if run_upgrade {
        vec![
            "pub".to_string(),
            "upgrade".to_string(),
            "--major-versions".to_string(),
            "--dry-run".to_string(),
        ]
    } else {
        vec![
            "pub".to_string(),
            "get".to_string(),
            "--dry-run".to_string(),
        ]
    };

    println!(
        "{} {}",
        gray(&format_timestamp()),
        cyan("Testing dependency compatibility via dry-run...")
    );

    let command_args = resolved_command.with_args(&dry_run_args);
    if verbose {
        println!(
            "{}",
            gray(&format!(
                "Executing: {}",
                resolved_command.describe(&command_args)
            ))
        );
    }

    let output = match tokio::process::Command::new(&resolved_command.executable)
        .args(&command_args)
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("{}", red(&format!("Failed to execute command: {error}")));
            return 1;
        }
    };

    if output.status.success() {
        println!();
        println!(
            "{}",
            green("✓ All dependencies are compatible and can be resolved cleanly!")
        );
        if run_upgrade {
            println!(
                "{}",
                gray(&format!(
                    "You can safely run \"{} pub upgrade --major-versions\".",
                    resolved_command.executable
                ))
            );
        }
        return 0;
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);
    let combined_output = format!("{stdout_str}\n{stderr_str}");

    let report = parse_pub_solver_error(&combined_output);
    display_diagnosis_report(&report, Some(&combined_output));
    0
}

/// Formats and outputs the structured diagnosis report to standard output.
pub fn display_diagnosis_report(report: &PubDiagnosisReport, raw_fallback: Option<&str>) {
    println!();
    println!(
        "{}",
        cyan("═══════════════════════════════════════════════════════════")
    );
    println!("{}", cyan("  🔍 Pub Dependency Conflict Diagnosis"));
    println!(
        "{}",
        cyan("═══════════════════════════════════════════════════════════")
    );

    if !report.has_issues() {
        println!(
            "{}",
            yellow("Could not detect specific structural conflicts in solver output.")
        );
        if let Some(fallback) = raw_fallback {
            let trimmed = fallback.trim();
            if !trimmed.is_empty() {
                println!();
                println!("{}", gray("Raw output:"));
                println!("{trimmed}");
            }
        }
        println!(
            "{}",
            cyan("═══════════════════════════════════════════════════════════")
        );
        return;
    }

    if let Some(ref deadlock) = report.root_deadlock {
        println!();
        println!("{}", red("❌ Root Conflict Detected:"));
        println!(
            "   • Project {} depends on both {} ({}) and {} ({})",
            deadlock.project,
            deadlock.package_a,
            deadlock.version_range_a,
            deadlock.package_b,
            deadlock.version_range_b
        );
    }

    if !report.incompatibilities.is_empty() {
        println!();
        println!("{}", yellow("⚡ Incompatible Package Combinations:"));
        for pair in &report.incompatibilities {
            println!(
                "   • {} ({}) is incompatible with {} ({})",
                pair.package_a, pair.version_range_a, pair.package_b, pair.version_range_b
            );
        }
    }

    if !report.pinned_packages.is_empty() {
        println!();
        println!("{}", cyan("📌 Pinned SDK Packages:"));
        for pinned in &report.pinned_packages {
            println!(
                "   • {} is pinned to {} by {} ({} SDK)",
                pinned.package, pinned.version, pinned.owner, pinned.sdk
            );
        }
    }

    if !report.sdk_constraints.is_empty() || report.current_dart_sdk_version.is_some() {
        println!();
        println!("{}", cyan("⚙️ Dart SDK Constraints:"));
        if let Some(ref sdk_version) = report.current_dart_sdk_version {
            println!("   • Current Dart SDK version: {sdk_version}");
        }
        for constraint in &report.sdk_constraints {
            println!(
                "   • {} ({}) requires Dart SDK {}",
                constraint.package, constraint.version, constraint.sdk_range
            );
        }
    }

    let recommendations = report.generate_recommendations();
    if !recommendations.is_empty() {
        println!();
        println!("{}", green("💡 Recommended Solutions:"));
        for (index, recommendation) in recommendations.iter().enumerate() {
            println!("   {}. {recommendation}", index + 1);
        }
    }

    println!();
    println!(
        "{}",
        cyan("═══════════════════════════════════════════════════════════")
    );
}

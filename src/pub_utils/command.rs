use crate::common::ansi::red;
use crate::flutter::command::FlutterCommand;
use crate::pub_utils::diagnoser::run_dependency_diagnosis;
use crate::pub_utils::sorter::sort_pubspec;

/// Routes and executes `fl pub` CLI subcommands.
pub async fn handle_pub_command(
    flutter_command: &FlutterCommand,
    arguments: &[String],
    verbose: bool,
) -> i32 {
    if arguments.is_empty() {
        eprintln!("{}", red("No pub subcommand specified"));
        eprintln!();
        eprintln!("Available subcommands:");
        eprintln!("  sort [options]        Sort dependencies in pubspec.yaml alphabetically");
        eprintln!(
            "  diagnose [options]    Diagnose dependency conflicts and PubGrub solver failures"
        );
        eprintln!();
        eprintln!("Subcommand options:");
        eprintln!("  sort:");
        eprintln!("    --create-backup     Create a backup file (pubspec.yaml.backup)");
        eprintln!("  diagnose:");
        eprintln!("    --upgrade           Diagnose major version upgrades (dry-run)");
        eprintln!("    --input <file>      Diagnose a saved solver output text file");
        return 64;
    }

    let subcommand = &arguments[0];

    if subcommand == "sort" {
        let mut create_backup = false;
        let mut index = 1;

        while index < arguments.len() {
            let current = &arguments[index];
            if current == "--create-backup" {
                create_backup = true;
                index += 1;
                continue;
            }

            eprintln!("{}", red(&format!("Unknown option: {current}")));
            eprintln!();
            eprintln!("Sort options:");
            eprintln!("  --create-backup   Create a backup file (pubspec.yaml.backup)");
            return 64;
        }

        match sort_pubspec(verbose, create_backup) {
            Ok(_) => 0,
            Err(_) => 1,
        }
    } else if subcommand == "diagnose" {
        let mut run_upgrade = false;
        let mut input_file_path = None;
        let mut index = 1;

        while index < arguments.len() {
            let current = &arguments[index];

            if current == "--upgrade" {
                run_upgrade = true;
                index += 1;
                continue;
            }

            if current == "--input" {
                if index + 1 >= arguments.len() {
                    eprintln!("{}", red("Expected file path after --input."));
                    return 64;
                }
                index += 1;
                input_file_path = Some(arguments[index].as_str());
                index += 1;
                continue;
            }

            if let Some(stripped) = current.strip_prefix("--input=") {
                input_file_path = Some(stripped);
                index += 1;
                continue;
            }

            eprintln!(
                "{}",
                red(&format!("Unknown option for pub diagnose: {current}"))
            );
            return 64;
        }

        run_dependency_diagnosis(Some(flutter_command), run_upgrade, input_file_path, verbose).await
    } else {
        eprintln!("{}", red(&format!("Unknown pub subcommand: {subcommand}")));
        eprintln!();
        eprintln!("Available subcommands:");
        eprintln!("  sort [options]        Sort dependencies in pubspec.yaml alphabetically");
        eprintln!(
            "  diagnose [options]    Diagnose dependency conflicts and PubGrub solver failures"
        );
        64
    }
}

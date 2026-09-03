use std::env;

use fl_rust::cli::args::{extract_run_command_args, parse_arguments};
use fl_rust::cli::usage::print_usage;
use fl_rust::common::ansi::{gray, red};
use fl_rust::common::constants::CLI_VERSION;
use fl_rust::devices::command::handle_device_command;
use fl_rust::flutter::command::resolve_flutter_command;
use fl_rust::flutter::runner::FlutterRunner;
use fl_rust::pub_utils::command::handle_pub_command;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let parsed = match parse_arguments(&args) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("{}", red(&error.message));
            eprintln!();
            print_usage();
            std::process::exit(64);
        }
    };

    if parsed.show_version {
        println!("fl version {CLI_VERSION}");
        return;
    }

    let verbose = parsed.verbose;
    let command = parsed.command;
    let command_args = parsed.command_args;

    if verbose {
        println!("{}", gray("Debug: Parsed arguments"));
        println!("{}", gray(&format!("  showHelp: {}", parsed.show_help)));
        println!(
            "{}",
            gray(&format!("  showVersion: {}", parsed.show_version))
        );
        println!("{}", gray(&format!("  verbose: {}", parsed.verbose)));
        println!("{}", gray(&format!("  command: {:?}", command)));
        println!("{}", gray(&format!("  commandArgs: {:?}", command_args)));
    }

    if parsed.show_help && command.is_none() {
        if verbose {
            println!("{}", gray("Debug: Showing fl help (no command)"));
        }
        print_usage();
        return;
    }

    let command_name = match command {
        Some(ref name) if !name.is_empty() => name.as_str(),
        _ => {
            if !parsed.show_help {
                eprintln!("{}", red("No command specified"));
                eprintln!();
            }
            if verbose {
                println!("{}", gray("Debug: No command provided"));
            }
            print_usage();
            std::process::exit(64);
        }
    };

    let flutter_command = resolve_flutter_command(None);

    if command_name == "run" {
        if verbose {
            println!("{}", gray("Debug: Executing run command"));
        }

        let run_args = match extract_run_command_args(&command_args) {
            Ok(run_args) => run_args,
            Err(error) => {
                eprintln!("{}", red(&error.message));
                eprintln!();
                print_usage();
                std::process::exit(64);
            }
        };

        let runner = FlutterRunner::new(
            run_args.cleaned_args,
            run_args.platform_override,
            run_args.filter_out_patterns,
            verbose,
            run_args.force_device_refresh,
            run_args.auto_yes,
            Some(flutter_command),
        );

        let exit_code = runner.run().await;
        std::process::exit(exit_code);
    }

    if command_name == "flutter" {
        if verbose {
            println!("{}", gray("Debug: Forwarding to flutter command"));
        }
        let exit_code = flutter_command
            .run_passthrough(&command_args, verbose)
            .await;
        std::process::exit(exit_code);
    }

    if command_name == "pub" {
        if verbose {
            println!("{}", gray("Debug: Executing pub command"));
        }
        let exit_code = handle_pub_command(&flutter_command, &command_args, verbose).await;
        std::process::exit(exit_code);
    }

    if command_name == "help" {
        if verbose {
            println!("{}", gray("Debug: Showing help command"));
        }
        print_usage();
        return;
    }

    if command_name == "device" {
        if verbose {
            println!("{}", gray("Debug: Executing device command"));
        }
        let exit_code = handle_device_command(&flutter_command, &command_args, verbose).await;
        std::process::exit(exit_code);
    }

    eprintln!("{}", red(&format!("Unknown command: {command_name}")));
    eprintln!();
    print_usage();
    std::process::exit(64);
}

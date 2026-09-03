use crate::common::ansi::{cyan, format_timestamp, gray, green, red, yellow};
use crate::devices::cache::{
    fetch_devices, load_cached_devices, remove_device_from_cache, save_device_cache,
};
use crate::devices::models::FlutterDevice;
use crate::flutter::command::FlutterCommand;

/// Handles `fl device` CLI subcommands.
pub async fn handle_device_command(
    flutter_command: &FlutterCommand,
    arguments: &[String],
    verbose: bool,
) -> i32 {
    if arguments.is_empty() {
        eprintln!("{}", red("No device subcommand specified"));
        eprintln!();
        eprintln!("Available subcommands:");
        eprintln!("  refresh           Refresh the device cache");
        eprintln!("  list [options]    List devices (use --force-device-refresh to refresh)");
        eprintln!("  rm <device-id>    Remove a device from the cache");
        return 64;
    }

    let subcommand = &arguments[0];

    if subcommand == "refresh" {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            cyan("Refreshing device cache...")
        );
        let devices = fetch_devices(flutter_command, verbose).await;
        if !devices.is_empty() {
            save_device_cache(&devices, verbose);
            println!(
                "{}",
                green(&format!(
                    "✓ Device cache updated with {} devices.",
                    devices.len()
                ))
            );
            for device in &devices {
                println!("  • {} ({})", device.name, device.id);
            }
            return 0;
        }

        println!("{}", yellow("No devices found."));
        return 0;
    }

    if subcommand == "list" {
        let mut force_refresh = false;
        for arg in &arguments[1..] {
            if arg == "--force-device-refresh" {
                force_refresh = true;
                break;
            }
        }

        if force_refresh {
            println!("{}", cyan("Refreshing device cache..."));
            let devices = fetch_devices(flutter_command, verbose).await;
            if !devices.is_empty() {
                save_device_cache(&devices, verbose);
            }
            print_devices(&devices);
            return 0;
        }

        if let Some(cached_devices) = load_cached_devices(verbose)
            && !cached_devices.is_empty()
        {
            print_devices(&cached_devices);
            return 0;
        }

        println!("{}", yellow("No cached devices found."));
        println!(
            "{}",
            gray(
                "Run \"fl device refresh\" or \"fl device list --force-device-refresh\" to update."
            )
        );
        return 0;
    }

    if subcommand == "rm" {
        if arguments.len() < 2 {
            eprintln!("{}", red("No device ID specified"));
            eprintln!("{}", gray("Usage: fl device rm <device-id>"));
            return 64;
        }

        let device_id = &arguments[1];
        let removed = remove_device_from_cache(device_id, verbose);
        if removed {
            println!(
                "{}",
                green(&format!("✓ Device {device_id} removed from cache."))
            );
            return 0;
        }

        println!(
            "{}",
            yellow(&format!("Device {device_id} not found in cache."))
        );
        return 0;
    }

    eprintln!(
        "{}",
        red(&format!("Unknown device subcommand: {subcommand}"))
    );
    64
}

/// Formats and displays a list of devices to standard output.
pub fn print_devices(devices: &[FlutterDevice]) {
    if devices.is_empty() {
        println!("{}", yellow("No devices available."));
        return;
    }
    println!("{}", green("Available devices:"));
    for device in devices {
        let platform = device.target_platform.as_deref().unwrap_or("");
        let sdk = device.sdk.as_deref().unwrap_or("");
        println!("  • {} ({}) - {platform} {sdk}", device.name, device.id);
    }
}

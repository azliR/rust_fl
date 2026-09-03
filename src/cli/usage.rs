use crate::common::ansi::cyan;

/// Prints CLI usage help information.
pub fn print_usage() {
    println!("fl - Enhanced Flutter CLI");
    println!();
    println!("Usage: fl [global-options] <command> [command-arguments]");
    println!();
    println!("Global options (must come before command):");
    println!("  -h, --help        Show this help message");
    println!("      --version     Show version information");
    println!("  -v, --verbose     Verbose output");
    println!();
    println!("Commands:");
    println!("  run [flutter args]    Launch Flutter with auto reload/log capture");
    println!(
        "      --platform <name>   Restrict device selection to one platform \
(android, ios, linux, macos, windows, web)"
    );
    println!("      --force-device-refresh   Bypass device cache and fetch fresh devices");
    println!("      -y, --yes            Auto-select the first device");
    println!("  pub <subcommand>      Pub-related utilities");
    println!("    sort [options]    Sort dependencies in pubspec.yaml alphabetically");
    println!("    diagnose [options] Diagnose dependency conflicts and solver errors");
    println!("  flutter <flutter args>  Pass through any command to the Flutter CLI");
    println!("  device <subcommand>   Device management commands");
    println!("    refresh           Manually refresh the device cache");
    println!("    list              List devices from cache (supports --force-device-refresh)");
    println!("    rm <device-id>    Remove a device from the cache");
    println!("  help                Show this message");
    println!();
    println!("Examples:");
    println!("  fl run                          # Run with defaults");
    println!("  fl run --help                   # Show Flutter run help");
    println!("  fl run --target lib/main_dev.dart   # Run specific target");
    println!("  fl run --flavor development --debug   # Run with flavor");
    println!("  fl run --platform ios              # Limit selection to iOS devices");
    println!("  fl run --force-device-refresh    # Refresh device list");
    println!("  fl -v run --target lib/main.dart    # Verbose mode");
    println!("  fl pub sort                       # Sort pubspec.yaml dependencies");
    println!("  fl pub diagnose                   # Diagnose dependency conflicts");
    println!("  fl pub diagnose --upgrade         # Check major version upgrade issues");
    println!("  fl device refresh                 # Refresh device cache");
    println!("  fl device list                    # List cached devices");
    println!("  fl device list --force-device-refresh # Refresh and list devices");
    println!("  fl --help                       # Show this message");
    println!("  fl --version                    # Show version");
    println!("  fl flutter doctor             # Run Flutter CLI commands directly");
    println!();
    println!("{}", cyan("Commands during execution:"));
    println!("  r - Hot reload");
    println!("  R - Hot restart");
    println!("  q - Quit");
    println!("  h - Help");
}

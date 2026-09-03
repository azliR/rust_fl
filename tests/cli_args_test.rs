use fl_rust::cli::args::{extract_run_command_args, normalize_platform_value, parse_arguments};

#[test]
fn test_parse_arguments_global_help() {
    let args = vec!["--help".to_string()];
    let parsed = parse_arguments(&args).expect("Should parse");
    assert!(parsed.show_help);
    assert!(!parsed.show_version);
    assert!(!parsed.verbose);
    assert_eq!(parsed.command, None);
    assert!(parsed.command_args.is_empty());

    let args_short = vec!["-h".to_string()];
    let parsed_short = parse_arguments(&args_short).expect("Should parse");
    assert!(parsed_short.show_help);
}

#[test]
fn test_parse_arguments_global_version() {
    let args = vec!["--version".to_string()];
    let parsed = parse_arguments(&args).expect("Should parse");
    assert!(parsed.show_version);
}

#[test]
fn test_parse_arguments_global_verbose_and_command() {
    let args = vec![
        "-v".to_string(),
        "run".to_string(),
        "--target".to_string(),
        "lib/main.dart".to_string(),
    ];
    let parsed = parse_arguments(&args).expect("Should parse");
    assert!(parsed.verbose);
    assert_eq!(parsed.command.as_deref(), Some("run"));
    assert_eq!(parsed.command_args, vec!["--target", "lib/main.dart"]);
}

#[test]
fn test_parse_arguments_unknown_global_option() {
    let args = vec!["--unknown".to_string(), "run".to_string()];
    let error = parse_arguments(&args).expect_err("Should fail");
    assert!(error.message.contains("Unknown global option: --unknown"));
}

#[test]
fn test_extract_run_command_args_options() {
    let args = vec![
        "--platform".to_string(),
        "ios".to_string(),
        "-y".to_string(),
        "--force-device-refresh".to_string(),
        "--flavor".to_string(),
        "dev".to_string(),
    ];
    let run_args = extract_run_command_args(&args).expect("Should extract");
    assert_eq!(run_args.platform_override.as_deref(), Some("ios"));
    assert!(run_args.auto_yes);
    assert!(run_args.force_device_refresh);
    assert_eq!(run_args.cleaned_args, vec!["--flavor", "dev"]);
}

#[test]
fn test_extract_run_command_args_platform_equals() {
    let args = vec!["--platform=android".to_string(), "--debug".to_string()];
    let run_args = extract_run_command_args(&args).expect("Should extract");
    assert_eq!(run_args.platform_override.as_deref(), Some("android"));
    assert_eq!(run_args.cleaned_args, vec!["--debug"]);
}

#[test]
fn test_extract_run_command_args_multiple_platforms_rejected() {
    let args = vec![
        "--platform".to_string(),
        "ios".to_string(),
        "--platform".to_string(),
        "android".to_string(),
    ];
    let error = extract_run_command_args(&args).expect_err("Should error");
    assert!(
        error
            .message
            .contains("Multiple --platform arguments are not allowed.")
    );
}

#[test]
fn test_extract_run_command_args_preserves_double_dash() {
    let args = vec![
        "-y".to_string(),
        "--".to_string(),
        "--platform".to_string(),
        "web".to_string(),
    ];
    let run_args = extract_run_command_args(&args).expect("Should extract");
    assert!(run_args.auto_yes);
    assert_eq!(run_args.platform_override, None);
    assert_eq!(run_args.cleaned_args, vec!["--", "--platform", "web"]);
}

#[test]
fn test_normalize_platform_value() {
    assert_eq!(normalize_platform_value("iOS").unwrap(), "ios");
    assert_eq!(normalize_platform_value(" Android ").unwrap(), "android");
    assert_eq!(normalize_platform_value("web").unwrap(), "web");
    assert!(normalize_platform_value("blackberry").is_err());
}

#[test]
fn test_extract_run_command_args_filter_out_options() {
    let args = vec![
        "--filter-out".to_string(),
        "gralloc4".to_string(),
        "--ignore-log=SMPTE".to_string(),
        "--target".to_string(),
        "lib/main.dart".to_string(),
    ];
    let run_args = extract_run_command_args(&args).expect("Should extract");
    assert_eq!(
        run_args.filter_out_patterns,
        vec!["gralloc4".to_string(), "SMPTE".to_string()]
    );
    assert_eq!(run_args.cleaned_args, vec!["--target", "lib/main.dart"]);
}

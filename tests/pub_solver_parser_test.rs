use fl_rust::pub_utils::solver_parser::parse_pub_solver_error;

#[test]
fn test_parse_pub_solver_deadlock_and_pins() {
    let output = r#"
The current Dart SDK version is 3.5.0.
Note: intl is pinned to version 0.19.0 by flutter_localizations from the flutter SDK.
So, because my_app depends on both intl ^0.20.0 and flutter_localizations from sdk, version solving failed.
"#;

    let report = parse_pub_solver_error(output);
    assert!(report.has_issues());
    assert_eq!(report.current_dart_sdk_version.as_deref(), Some("3.5.0"));

    assert_eq!(report.pinned_packages.len(), 1);
    let pin = &report.pinned_packages[0];
    assert_eq!(pin.package, "intl");
    assert_eq!(pin.version, "0.19.0");
    assert_eq!(pin.owner, "flutter_localizations");
    assert_eq!(pin.sdk, "flutter");

    let deadlock = report.root_deadlock.as_ref().expect("Should have deadlock");
    assert_eq!(deadlock.project, "my_app");
    assert_eq!(deadlock.package_a, "intl");
    assert_eq!(deadlock.version_range_a, "^0.20.0");
    assert_eq!(deadlock.package_b, "flutter_localizations");

    let recommendations = report.generate_recommendations();
    assert!(!recommendations.is_empty());
    assert!(
        recommendations
            .iter()
            .any(|r| r.contains("Flutter SDK is pinning: intl (0.19.0)"))
    );
}

#[test]
fn test_parse_sdk_constraints() {
    let output = r#"
Because my_pkg depends on modern_pkg 2.0.0 which requires SDK version >=3.7.0 <4.0.0, version solving failed.
"#;

    let report = parse_pub_solver_error(output);
    assert_eq!(report.sdk_constraints.len(), 1);
    assert_eq!(report.sdk_constraints[0].package, "modern_pkg");
    assert_eq!(report.sdk_constraints[0].version, "2.0.0");
    assert_eq!(report.sdk_constraints[0].sdk_range, ">=3.7.0 <4.0.0");
}

#[test]
fn test_parse_incompatibilities() {
    let output = r#"
foo 1.0.0 is incompatible with bar 2.0.0.
"#;

    let report = parse_pub_solver_error(output);
    assert_eq!(report.incompatibilities.len(), 1);
    assert_eq!(report.incompatibilities[0].package_a, "foo");
    assert_eq!(report.incompatibilities[0].version_range_a, "1.0.0");
    assert_eq!(report.incompatibilities[0].package_b, "bar");
    assert_eq!(report.incompatibilities[0].version_range_b, "2.0.0");
}

use fl_rust::pub_utils::sorter::sort_pubspec_content;

#[test]
fn test_sort_simple_dependencies() {
    let input = r#"name: sample_app
description: A sample app

dependencies:
  flutter:
    sdk: flutter
  path: ^1.9.0
  args: ^2.5.0

dev_dependencies:
  test: ^1.25.0
  lints: ^3.0.0
"#;

    let sorted = sort_pubspec_content(input).expect("Should sort");

    let expected = r#"name: sample_app
description: A sample app

dependencies:
  args: ^2.5.0
  flutter:
    sdk: flutter
  path: ^1.9.0

dev_dependencies:
  lints: ^3.0.0
  test: ^1.25.0
"#;

    assert_eq!(sorted, expected);
}

#[test]
fn test_sort_complex_multiline_dependencies() {
    let input = r#"dependencies:
  zebra: ^1.0.0
  alpha:
    git:
      url: https://github.com/example/alpha.git
      ref: main
  beta:
    path: ../beta
"#;

    let sorted = sort_pubspec_content(input).expect("Should sort");

    let expected = "dependencies:\n  alpha:\n    git:\n      url: https://github.com/example/alpha.git\n      ref: main\n  beta:\n    path: ../beta\n  zebra: ^1.0.0\n";

    assert_eq!(sorted, expected);
}

#[test]
fn test_preserves_surrounding_sections() {
    let input = r#"name: my_pkg
version: 1.0.0

environment:
  sdk: ^3.5.0

dependencies:
  zoo: ^1.0.0
  ant: ^2.0.0

flutter:
  uses-material-design: true
"#;

    let sorted = sort_pubspec_content(input).expect("Should sort");

    assert!(sorted.starts_with("name: my_pkg\nversion: 1.0.0"));
    assert!(sorted.contains("dependencies:\n  ant: ^2.0.0\n  zoo: ^1.0.0"));
    assert!(sorted.ends_with("flutter:\n  uses-material-design: true\n"));
}

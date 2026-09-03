use fl_rust::devices::filter::DirectoryPlatformFilter;
use fl_rust::devices::models::FlutterDevice;

#[test]
fn test_filter_matches_platform_and_sdk() {
    let filter = DirectoryPlatformFilter::new(
        vec!["ios".to_string(), "android".to_string()],
        vec!["iOS".to_string(), "Android".to_string()],
    );

    let ios_device = FlutterDevice {
        id: "ios-sim".to_string(),
        name: "iPhone 16".to_string(),
        target_platform: Some("ios".to_string()),
        sdk: Some("iOS 18.0".to_string()),
    };

    let android_device = FlutterDevice {
        id: "pixel".to_string(),
        name: "Pixel 9".to_string(),
        target_platform: Some("android-arm64".to_string()),
        sdk: Some("Android 14".to_string()),
    };

    let macos_device = FlutterDevice {
        id: "macos".to_string(),
        name: "macOS".to_string(),
        target_platform: Some("darwin".to_string()),
        sdk: Some("macOS 15".to_string()),
    };

    assert!(filter.matches(&ios_device));
    assert!(filter.matches(&android_device));
    assert!(!filter.matches(&macos_device));
}

#[test]
fn test_filter_describe() {
    let single = DirectoryPlatformFilter::new(vec!["ios".to_string()], vec!["iOS".to_string()]);
    assert_eq!(single.describe(), "iOS");

    let double = DirectoryPlatformFilter::new(
        vec!["android".to_string(), "ios".to_string()],
        vec!["Android".to_string(), "iOS".to_string()],
    );
    assert_eq!(double.describe(), "Android and iOS");

    let triple = DirectoryPlatformFilter::new(
        vec!["android".to_string(), "ios".to_string(), "web".to_string()],
        vec!["Android".to_string(), "iOS".to_string(), "Web".to_string()],
    );
    assert_eq!(triple.describe(), "Android, iOS, and Web");
}

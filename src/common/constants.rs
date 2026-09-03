/// Current CLI version string.
pub const CLI_VERSION: &str = "0.19.0";

/// Number of days after which unpicked devices are pruned from cache.
pub const STALE_DEVICE_DAYS: i64 = 30;

/// Mapping of platform directory names to display names.
pub const PLATFORM_DIRECTORY_MAPPINGS: &[(&str, &str)] = &[
    ("android", "Android"),
    ("ios", "iOS"),
    ("windows", "Windows"),
    ("linux", "Linux"),
    ("macos", "macOS"),
    ("web", "Web"),
];

/// Returns the formatted display name for a platform key if recognized.
pub fn get_platform_display_name(platform_key: &str) -> Option<&'static str> {
    for &(directory_name, display_name) in PLATFORM_DIRECTORY_MAPPINGS {
        if directory_name == platform_key {
            return Some(display_name);
        }
    }
    None
}

/// Checks whether a platform name is supported.
pub fn is_supported_platform(platform_key: &str) -> bool {
    get_platform_display_name(platform_key).is_some()
}

/// Returns a comma-separated list of supported platform names.
pub fn supported_platforms_string() -> String {
    PLATFORM_DIRECTORY_MAPPINGS
        .iter()
        .map(|&(directory_name, _)| directory_name)
        .collect::<Vec<_>>()
        .join(", ")
}

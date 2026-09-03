use std::path::Path;

use crate::common::ansi::gray;
use crate::common::constants::{PLATFORM_DIRECTORY_MAPPINGS, get_platform_display_name};
use crate::devices::models::FlutterDevice;

/// Filters devices based on platform directories present in the project or explicit user override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryPlatformFilter {
    pub segments: Vec<String>,
    pub labels: Vec<String>,
}

impl DirectoryPlatformFilter {
    pub fn new(segments: Vec<String>, labels: Vec<String>) -> Self {
        Self { segments, labels }
    }

    /// Checks if a device's platform or SDK matches any of the filter segments.
    pub fn matches(&self, device: &FlutterDevice) -> bool {
        let target = device
            .target_platform
            .as_deref()
            .unwrap_or("")
            .to_lowercase();
        let sdk = device.sdk.as_deref().unwrap_or("").to_lowercase();

        for segment in &self.segments {
            if target.contains(segment) || sdk.contains(segment) {
                return true;
            }
        }
        false
    }

    /// Formats a human-readable description of the matched platforms.
    pub fn describe(&self) -> String {
        if self.labels.is_empty() {
            return "detected platforms".to_string();
        }
        if self.labels.len() == 1 {
            return self.labels[0].clone();
        }
        if self.labels.len() == 2 {
            return format!("{} and {}", self.labels[0], self.labels[1]);
        }
        let all_except_last = self.labels[..self.labels.len() - 1].join(", ");
        format!("{}, and {}", all_except_last, self.labels.last().unwrap())
    }
}

/// Resolves platform filter from explicit override or project directories.
pub fn determine_platform_filter(
    platform_override: Option<&str>,
) -> Option<DirectoryPlatformFilter> {
    if let Some(override_platform) = platform_override {
        let label = get_platform_display_name(override_platform)
            .unwrap_or(override_platform)
            .to_string();
        return Some(DirectoryPlatformFilter::new(
            vec![override_platform.to_string()],
            vec![label],
        ));
    }
    determine_directory_platform_filter()
}

/// Detects which platform directories exist in current working directory.
pub fn determine_directory_platform_filter() -> Option<DirectoryPlatformFilter> {
    let mut segments = Vec::new();
    let mut labels = Vec::new();

    for &(directory_name, display_name) in PLATFORM_DIRECTORY_MAPPINGS {
        if Path::new(directory_name).is_dir() {
            segments.push(directory_name.to_string());
            labels.push(display_name.to_string());
        }
    }

    if segments.is_empty() {
        return None;
    }

    Some(DirectoryPlatformFilter::new(segments, labels))
}

/// Filters device list using the platform filter.
pub fn filter_devices_by_directory(
    devices: Vec<FlutterDevice>,
    filter: Option<&DirectoryPlatformFilter>,
    verbose: bool,
) -> Vec<FlutterDevice> {
    let Some(filter) = filter else {
        return devices;
    };

    let filtered: Vec<FlutterDevice> = devices
        .into_iter()
        .filter(|device| filter.matches(device))
        .collect();

    if !verbose {
        return filtered;
    }

    if filtered.is_empty() {
        println!(
            "{}",
            gray(&format!(
                "No devices matched the requested {} platform(s).",
                filter.describe()
            ))
        );
        return filtered;
    }

    println!(
        "{}",
        gray(&format!(
            "Filtering to {} devices ({} available).",
            filter.describe(),
            filtered.len()
        ))
    );

    filtered
}

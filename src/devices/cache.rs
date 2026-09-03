use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use serde_json::Value;

use crate::common::ansi::{gray, red};
use crate::common::constants::STALE_DEVICE_DAYS;
use crate::devices::models::{DeviceCachePayload, DeviceRecord, FlutterDevice};
use crate::flutter::command::FlutterCommand;

/// Resolves the base cache path based on environment variables or platform conventions.
pub fn resolve_device_cache_base_path() -> Option<PathBuf> {
    if let Ok(override_dir) = std::env::var("FL_DEVICE_CACHE_DIR")
        && !override_dir.trim().is_empty()
    {
        return Some(PathBuf::from(override_dir));
    }

    #[cfg(windows)]
    {
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            if !user_profile.trim().is_empty() {
                return Some(PathBuf::from(user_profile).join(".fl"));
            }
        }
        dirs::home_dir().map(|path| path.join(".fl"))
    }

    #[cfg(not(windows))]
    {
        if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME")
            && !xdg_cache.trim().is_empty()
        {
            return Some(PathBuf::from(xdg_cache).join("fl"));
        }
        if let Some(cache_dir) = dirs::cache_dir() {
            return Some(cache_dir.join("fl"));
        }
        dirs::home_dir().map(|path| path.join(".cache").join("fl"))
    }
}

/// Resolves and ensures the device cache directory exists.
pub fn resolve_device_cache_directory() -> Option<PathBuf> {
    let base_path = resolve_device_cache_base_path()?;
    if !base_path.exists() && fs::create_dir_all(&base_path).is_err() {
        return None;
    }
    Some(base_path)
}

/// Resolves the JSON device cache file path.
pub fn resolve_device_cache_file() -> Option<PathBuf> {
    let directory = resolve_device_cache_directory()?;
    Some(directory.join("device-cache.json"))
}

/// Fetches connected devices from Flutter CLI using machine-readable JSON output.
pub async fn fetch_devices(flutter_command: &FlutterCommand, verbose: bool) -> Vec<FlutterDevice> {
    let command_args = flutter_command.with_args(&["devices".to_string(), "--machine".to_string()]);
    let mut command = tokio::process::Command::new(&flutter_command.executable);
    command.args(&command_args);

    let output = match command.output().await {
        Ok(output) => output,
        Err(error) => {
            if verbose {
                eprintln!("{}", red(&format!("Failed to list devices: {error}")));
            }
            return Vec::new();
        }
    };

    if !output.status.success() {
        if verbose {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            eprintln!("{}", red(&format!("Failed to list devices: {stderr_str}")));
        }
        return Vec::new();
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    parse_devices_from_output(&stdout_str, verbose)
}

/// Loads cached device usage records from disk, automatically pruning stale records.
pub fn load_device_records(verbose: bool) -> Option<HashMap<String, DeviceRecord>> {
    let file_path = resolve_device_cache_file()?;
    if !file_path.exists() {
        return None;
    }

    let content = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(error) => {
            if verbose {
                eprintln!("{}", red(&format!("Failed to read device cache: {error}")));
            }
            return None;
        }
    };

    let payload: DeviceCachePayload = match serde_json::from_str(&content) {
        Ok(payload) => payload,
        Err(error) => {
            if verbose {
                eprintln!("{}", red(&format!("Failed to load device cache: {error}")));
            }
            return None;
        }
    };

    let mut records = HashMap::new();
    let now = Utc::now();
    let stale_duration = Duration::days(STALE_DEVICE_DAYS);

    for (device_id, record) in payload.devices {
        if let Some(last_picked) = record.last_picked_at {
            let age = now.signed_duration_since(last_picked);
            if age > stale_duration {
                if verbose {
                    println!(
                        "{}",
                        gray(&format!(
                            "Removing stale device: {} (not picked for {} days)",
                            record.name,
                            age.num_days()
                        ))
                    );
                }
                continue;
            }
        }
        records.insert(device_id, record);
    }

    Some(records)
}

/// Persists device records to the JSON cache file.
pub fn save_device_records(records: &HashMap<String, DeviceRecord>, verbose: bool) {
    let file_path = match resolve_device_cache_file() {
        Some(path) => path,
        None => return,
    };

    let payload = DeviceCachePayload {
        devices: records.clone(),
    };

    let json_bytes = match serde_json::to_string_pretty(&payload) {
        Ok(bytes) => bytes,
        Err(error) => {
            if verbose {
                eprintln!(
                    "{}",
                    red(&format!("Failed to encode device cache: {error}"))
                );
            }
            return;
        }
    };

    if let Err(error) = fs::write(&file_path, json_bytes)
        && verbose
    {
        eprintln!("{}", red(&format!("Failed to write device cache: {error}")));
    }
}

/// Merges freshly fetched devices into existing cached records.
pub fn merge_devices_into_records(
    fetched_devices: &[FlutterDevice],
    verbose: bool,
) -> HashMap<String, DeviceRecord> {
    let mut existing = load_device_records(verbose).unwrap_or_default();
    let now = Utc::now();

    for device in fetched_devices {
        if let Some(old_record) = existing.get_mut(&device.id) {
            old_record.name = device.name.clone();
            old_record.target_platform = device.target_platform.clone();
            old_record.sdk = device.sdk.clone();
            old_record.last_seen_at = Some(now);
            continue;
        }

        existing.insert(
            device.id.clone(),
            DeviceRecord::from_device(device.clone(), None, Some(now), HashMap::new()),
        );
    }

    existing
}

/// Records a device selection by updating global and project-specific timestamps.
pub fn record_device_pick(device_id: &str, project_path: Option<&str>, verbose: bool) {
    let mut records = load_device_records(verbose).unwrap_or_default();
    let Some(old_record) = records.get_mut(device_id) else {
        return;
    };

    let now = Utc::now();
    old_record.last_picked_at = Some(now);
    if let Some(path) = project_path {
        old_record
            .project_last_picked_at
            .insert(path.to_string(), now);
    }

    save_device_records(&records, verbose);
}

/// Removes a device from the disk cache.
pub fn remove_device_from_cache(device_id: &str, verbose: bool) -> bool {
    let mut records = match load_device_records(verbose) {
        Some(records) => records,
        None => return false,
    };

    if records.remove(device_id).is_none() {
        return false;
    }

    save_device_records(&records, verbose);
    true
}

/// Loads cached devices for display and fallback.
pub fn load_cached_devices(verbose: bool) -> Option<Vec<FlutterDevice>> {
    let records = load_device_records(verbose)?;
    if records.is_empty() {
        return None;
    }
    Some(records.values().map(DeviceRecord::device).collect())
}

/// Saves devices into the cache records.
pub fn save_device_cache(devices: &[FlutterDevice], verbose: bool) {
    let records = merge_devices_into_records(devices, verbose);
    save_device_records(&records, verbose);
}

/// Parses the output of flutter devices --machine.
pub fn parse_devices_from_output(output: &str, verbose: bool) -> Vec<FlutterDevice> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if let Ok(decoded) = serde_json::from_str::<Value>(trimmed) {
        return extract_devices_from_value(&decoded);
    }

    let mut devices = Vec::new();
    for line in trimmed.lines() {
        let line_trimmed = line.trim();
        if line_trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<Value>(line_trimmed) {
            Ok(decoded) => devices.extend(extract_devices_from_value(&decoded)),
            Err(_) => {
                if verbose {
                    eprintln!(
                        "{}",
                        red(&format!("Skipping malformed device entry: {line_trimmed}"))
                    );
                }
            }
        }
    }

    devices
}

/// Extracts devices from decoded JSON lists or maps.
pub fn extract_devices_from_value(value: &Value) -> Vec<FlutterDevice> {
    let mut devices = Vec::new();

    fn parse_single_device(obj: &serde_json::Map<String, Value>) -> Option<FlutterDevice> {
        let id = obj.get("id")?.as_str()?.to_string();
        let name = obj.get("name")?.as_str()?.to_string();
        let target_platform = obj
            .get("targetPlatform")
            .and_then(Value::as_str)
            .map(str::to_string);
        let sdk = obj.get("sdk").and_then(Value::as_str).map(str::to_string);

        Some(FlutterDevice {
            id,
            name,
            target_platform,
            sdk,
        })
    }

    match value {
        Value::Array(items) => {
            for item in items {
                if let Some(map) = item.as_object()
                    && let Some(device) = parse_single_device(map)
                {
                    devices.push(device);
                }
            }
        }
        Value::Object(map) => {
            if let Some(Value::Array(device_list)) = map.get("devices") {
                for item in device_list {
                    if let Some(device_map) = item.as_object()
                        && let Some(device) = parse_single_device(device_map)
                    {
                        devices.push(device);
                    }
                }
            } else if let Some(Value::Object(device_map)) = map.get("device") {
                if let Some(device) = parse_single_device(device_map) {
                    devices.push(device);
                }
            } else if let Some(device) = parse_single_device(map) {
                devices.push(device);
            }
        }
        _ => {}
    }

    devices
}

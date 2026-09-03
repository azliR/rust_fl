use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a target Flutter device detected by Flutter or loaded from cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlutterDevice {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk: Option<String>,
}

/// Tracks device usage statistics for recency ranking and staleness pruning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_picked_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub project_last_picked_at: HashMap<String, DateTime<Utc>>,
}

impl DeviceRecord {
    /// Constructs a DeviceRecord from a FlutterDevice.
    pub fn from_device(
        device: FlutterDevice,
        last_picked_at: Option<DateTime<Utc>>,
        last_seen_at: Option<DateTime<Utc>>,
        project_last_picked_at: HashMap<String, DateTime<Utc>>,
    ) -> Self {
        Self {
            id: device.id,
            name: device.name,
            target_platform: device.target_platform,
            sdk: device.sdk,
            last_picked_at,
            last_seen_at,
            project_last_picked_at,
        }
    }

    /// Extracts the FlutterDevice descriptor from this record.
    pub fn device(&self) -> FlutterDevice {
        FlutterDevice {
            id: self.id.clone(),
            name: self.name.clone(),
            target_platform: self.target_platform.clone(),
            sdk: self.sdk.clone(),
        }
    }
}

/// Tracks index changes when devices are refreshed during selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSelectionChanges {
    pub removed_indexes: Vec<usize>,
    pub added_devices: Vec<FlutterDevice>,
}

impl DeviceSelectionChanges {
    pub fn has_changes(&self) -> bool {
        !self.removed_indexes.is_empty() || !self.added_devices.is_empty()
    }
}

/// Root JSON container for device cache file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceCachePayload {
    pub devices: HashMap<String, DeviceRecord>,
}

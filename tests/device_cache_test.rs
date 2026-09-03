use std::collections::HashMap;

use chrono::{Duration, Utc};
use fl_rust::devices::cache::parse_devices_from_output;
use fl_rust::devices::models::{DeviceCachePayload, DeviceRecord, FlutterDevice};

#[test]
fn test_parse_devices_machine_json() {
    let json_output = r#"[
        {
            "id": "device-1",
            "name": "Pixel 8",
            "targetPlatform": "android-arm64",
            "sdk": "Android 14"
        },
        {
            "id": "device-2",
            "name": "iPhone 15",
            "targetPlatform": "ios",
            "sdk": "iOS 17"
        }
    ]"#;

    let devices = parse_devices_from_output(json_output, false);
    assert_eq!(devices.len(), 2);
    assert_eq!(devices[0].id, "device-1");
    assert_eq!(devices[0].name, "Pixel 8");
    assert_eq!(devices[1].id, "device-2");
    assert_eq!(devices[1].name, "iPhone 15");
}

#[test]
fn test_device_record_serialization_roundtrip() {
    let now = Utc::now();
    let mut project_picks = HashMap::new();
    project_picks.insert("/path/to/project".to_string(), now);

    let record = DeviceRecord {
        id: "test-device".to_string(),
        name: "Test Device".to_string(),
        target_platform: Some("ios".to_string()),
        sdk: Some("17.0".to_string()),
        last_picked_at: Some(now),
        last_seen_at: Some(now),
        project_last_picked_at: project_picks,
    };

    let mut payload = DeviceCachePayload::default();
    payload.devices.insert("test-device".to_string(), record);

    let serialized = serde_json::to_string(&payload).expect("Should serialize");
    let deserialized: DeviceCachePayload =
        serde_json::from_str(&serialized).expect("Should deserialize");

    let restored = deserialized.devices.get("test-device").unwrap();
    assert_eq!(restored.id, "test-device");
    assert_eq!(restored.name, "Test Device");
    assert_eq!(restored.target_platform.as_deref(), Some("ios"));
    assert!(
        restored
            .project_last_picked_at
            .contains_key("/path/to/project")
    );
}

#[test]
fn test_stale_device_pruning_logic() {
    let now = Utc::now();
    let stale_date = now - Duration::days(31);
    let fresh_date = now - Duration::days(5);

    let stale_record = DeviceRecord::from_device(
        FlutterDevice {
            id: "stale".to_string(),
            name: "Stale Device".to_string(),
            target_platform: None,
            sdk: None,
        },
        Some(stale_date),
        Some(stale_date),
        HashMap::new(),
    );

    let fresh_record = DeviceRecord::from_device(
        FlutterDevice {
            id: "fresh".to_string(),
            name: "Fresh Device".to_string(),
            target_platform: None,
            sdk: None,
        },
        Some(fresh_date),
        Some(fresh_date),
        HashMap::new(),
    );

    let mut records = HashMap::new();
    records.insert("stale".to_string(), stale_record);
    records.insert("fresh".to_string(), fresh_record);

    let stale_duration = Duration::days(30);
    records.retain(|_, record| {
        if let Some(picked) = record.last_picked_at {
            now.signed_duration_since(picked) <= stale_duration
        } else {
            true
        }
    });

    assert_eq!(records.len(), 1);
    assert!(records.contains_key("fresh"));
    assert!(!records.contains_key("stale"));
}

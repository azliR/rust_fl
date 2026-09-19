use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{IsTerminal, stdin};

use chrono::{DateTime, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::common::ansi::{cyan, format_timestamp, gray, red, yellow};
use crate::common::terminal::{RawModeGuard, term_print, term_println};
use crate::devices::cache::{
    fetch_devices, load_device_records, merge_devices_into_records, record_device_pick,
    save_device_cache, save_device_records,
};
use crate::devices::filter::{
    DirectoryPlatformFilter, determine_platform_filter, filter_devices_by_directory,
};
use crate::devices::models::{DeviceRecord, DeviceSelectionChanges, FlutterDevice};
use crate::flutter::command::FlutterCommand;

/// Tracks active status of an interactive device selection session.
pub struct SelectionSession {
    pub is_active: bool,
}

impl Default for SelectionSession {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionSession {
    pub fn new() -> Self {
        Self { is_active: true }
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
}

/// Manages indexed numeric slots for devices displayed in the CLI prompt.
pub struct DeviceSelectionContext {
    slots: BTreeMap<usize, FlutterDevice>,
    index_by_id: HashMap<String, usize>,
    missing_indexes: HashSet<usize>,
    next_index: usize,
}

impl Default for DeviceSelectionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceSelectionContext {
    pub fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            index_by_id: HashMap::new(),
            missing_indexes: HashSet::new(),
            next_index: 1,
        }
    }

    pub fn initialize(&mut self, devices: &[FlutterDevice]) {
        self.slots.clear();
        self.index_by_id.clear();
        self.missing_indexes.clear();
        self.next_index = 1;

        for device in devices {
            self.slots.insert(self.next_index, device.clone());
            self.index_by_id.insert(device.id.clone(), self.next_index);
            self.next_index += 1;
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (&usize, &FlutterDevice)> {
        self.slots.iter()
    }

    pub fn contains_index(&self, index: usize) -> bool {
        self.slots.contains_key(&index)
    }

    pub fn device_for_index(&self, index: usize) -> Option<&FlutterDevice> {
        self.slots.get(&index)
    }

    pub fn is_missing_index(&self, index: usize) -> bool {
        self.missing_indexes.contains(&index)
    }

    pub fn match_by_name_or_id(&self, candidate: &str) -> Option<&FlutterDevice> {
        let lower = candidate.to_lowercase();
        self.slots
            .values()
            .find(|device| device.id.to_lowercase() == lower || device.name.to_lowercase() == lower)
    }

    pub fn refresh(&mut self, new_devices: &[FlutterDevice]) -> DeviceSelectionChanges {
        let mut removed = Vec::new();
        let mut added = Vec::new();
        let new_ids: HashSet<String> = new_devices.iter().map(|d| d.id.clone()).collect();

        let existing_ids: HashSet<String> = self.index_by_id.keys().cloned().collect();
        let removed_ids: HashSet<_> = existing_ids.difference(&new_ids).collect();

        for id in removed_ids {
            if let Some(index) = self.index_by_id.remove(id) {
                self.slots.remove(&index);
                self.missing_indexes.insert(index);
                removed.push(index);
            }
        }

        for device in new_devices {
            if let Some(&existing_index) = self.index_by_id.get(&device.id) {
                self.slots.insert(existing_index, device.clone());
                continue;
            }

            self.slots.insert(self.next_index, device.clone());
            self.index_by_id.insert(device.id.clone(), self.next_index);
            added.push(device.clone());
            self.next_index += 1;
        }

        DeviceSelectionChanges {
            removed_indexes: removed,
            added_devices: added,
        }
    }
}

/// Checks if device identifier argument was already forwarded.
pub fn has_device_id_flag(forwarded_args: &[String]) -> bool {
    for arg in forwarded_args {
        if arg == "-d" || arg == "--device-id" {
            return true;
        }
        if arg.starts_with("-d") && arg.len() > 2 {
            return true;
        }
        if arg.starts_with("--device-id=") {
            return true;
        }
    }
    false
}

/// Prints available device choices numbered by slot.
pub fn print_device_choices(selection: &DeviceSelectionContext) {
    term_println("");
    term_println("Connected devices:");
    for (index, device) in selection.entries() {
        let platform = device.target_platform.as_deref().unwrap_or("unknown");
        let sdk_suffix = match &device.sdk {
            Some(sdk) if !sdk.is_empty() => format!(" • {sdk}"),
            _ => String::new(),
        };
        term_println(&format!(
            "[{index}]: {} ({}) • {platform}{sdk_suffix}",
            device.name, device.id
        ));
    }
    term_println("");
}

/// Refreshes device list once during an interactive selection session.
pub async fn refresh_devices_once(
    flutter_command: &FlutterCommand,
    filter: Option<&DirectoryPlatformFilter>,
    selection: &mut DeviceSelectionContext,
    started_from_cache: bool,
    session: &SelectionSession,
    verbose: bool,
) {
    let devices = fetch_devices(flutter_command, verbose).await;
    if devices.is_empty() {
        term_println(&yellow("No devices detected on refresh."));
        return;
    }

    save_device_cache(&devices, verbose);
    let filtered = filter_devices_by_directory(devices, filter, verbose);
    let changes = selection.refresh(&filtered);

    if !session.is_active || !changes.has_changes() {
        if !started_from_cache {
            term_println(&gray("Device list is unchanged."));
        }
        return;
    }

    term_println("");
    term_println(&yellow("Device list updated:"));
    print_device_choices(selection);
}

/// Determines whether a key event corresponds to confirming selection with Enter.
pub fn is_enter_key(key: &KeyEvent) -> bool {
    if matches!(
        key.code,
        KeyCode::Enter | KeyCode::Char('\n') | KeyCode::Char('\r')
    ) {
        return true;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(
            key.code,
            KeyCode::Char('j') | KeyCode::Char('m') | KeyCode::Char('J') | KeyCode::Char('M')
        )
    {
        return true;
    }

    false
}

/// Prompts user to pick a device interactively.
pub async fn prompt_device_selection(
    selection: &mut DeviceSelectionContext,
    flutter_command: &FlutterCommand,
    filter: Option<&DirectoryPlatformFilter>,
    session: &mut SelectionSession,
    started_from_cache: bool,
    verbose: bool,
) -> Option<String> {
    let use_single_key = stdin().is_terminal();

    loop {
        term_print("Please choose one (Enter=1, \"q\"=quit, \"r\"=refresh): ");

        if use_single_key {
            let _raw_guard = RawModeGuard::enter();
            let key_event = loop {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        term_println(&cyan("\n👋 Quitting..."));
                        std::process::exit(130);
                    }
                    break key;
                }
            };
            drop(_raw_guard);

            if is_enter_key(&key_event) {
                if selection.contains_index(1) {
                    term_println("");
                    return Some(selection.device_for_index(1).unwrap().id.clone());
                }
                continue;
            }

            match key_event.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    term_println(&cyan("\n👋 Quitting..."));
                    std::process::exit(0);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    term_println(&cyan("\nRefreshing device list..."));
                    refresh_devices_once(
                        flutter_command,
                        filter,
                        selection,
                        started_from_cache,
                        session,
                        verbose,
                    )
                    .await;
                    continue;
                }
                KeyCode::Char(digit) if digit.is_ascii_digit() => {
                    let index = (digit as u8 - b'0') as usize;
                    if selection.contains_index(index) {
                        term_println("");
                        return Some(selection.device_for_index(index).unwrap().id.clone());
                    }
                    if selection.is_missing_index(index) {
                        term_println(&red(&format!(
                            "\nDevice {index} is no longer available; please choose another device."
                        )));
                        continue;
                    }
                }
                _ => {}
            }

            term_println(&red(
                "\nInvalid selection. Enter a device number, or \"q\" to quit.",
            ));
            continue;
        }

        let mut line = String::new();
        if stdin().read_line(&mut line).is_err() {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if selection.contains_index(1) {
                term_println("");
                return Some(selection.device_for_index(1).unwrap().id.clone());
            }
            continue;
        }

        let lower = trimmed.to_lowercase();
        if lower == "q" {
            term_println(&cyan("\n👋 Quitting..."));
            std::process::exit(0);
        }

        if lower == "r" {
            term_println(&cyan("\nRefreshing device list..."));
            refresh_devices_once(
                flutter_command,
                filter,
                selection,
                started_from_cache,
                session,
                verbose,
            )
            .await;
            continue;
        }

        if let Ok(index) = trimmed.parse::<usize>() {
            if selection.contains_index(index) {
                term_println("");
                return Some(selection.device_for_index(index).unwrap().id.clone());
            }
            if selection.is_missing_index(index) {
                term_println(&red(&format!(
                    "\nDevice {index} is no longer available; please choose another device."
                )));
                continue;
            }
        }

        if let Some(device) = selection.match_by_name_or_id(trimmed) {
            term_println("");
            return Some(device.id.clone());
        }

        term_println(&red(
            "\nInvalid selection. Enter a device number or its name/ID, or \"q\" to quit.",
        ));
    }
}

/// Resolves target device ID from flags, ranking, cache, or interactive prompt.
pub async fn resolve_device_id(
    flutter_command: &FlutterCommand,
    forwarded_args: &[String],
    platform_override: Option<&str>,
    force_device_refresh: bool,
    auto_yes: bool,
    verbose: bool,
) -> Option<String> {
    if has_device_id_flag(forwarded_args) {
        if verbose {
            term_println(&gray(
                "Device flag already provided; skipping device selection.",
            ));
        }
        return None;
    }

    let filter = determine_platform_filter(platform_override);
    let mut records = load_device_records(verbose).unwrap_or_default();
    let mut using_cached_devices = !records.is_empty();

    if !force_device_refresh && using_cached_devices {
        term_println(&format!(
            "{} {}",
            gray(&format_timestamp()),
            gray("Using cached device list (press \"r\" to refresh).")
        ));
    }

    if force_device_refresh || !using_cached_devices {
        term_println(&format!(
            "{} {}",
            gray(&format_timestamp()),
            gray("Fetching device list...")
        ));
        let fetched_devices = fetch_devices(flutter_command, verbose).await;
        if !fetched_devices.is_empty() {
            records = merge_devices_into_records(&fetched_devices, verbose);
            save_device_records(&records, verbose);
            using_cached_devices = false;
        }
    }

    let project_path = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(str::to_string))
        .unwrap_or_default();

    let mut ranked_records: Vec<DeviceRecord> = records.into_values().collect();
    if let Some(ref filter_instance) = filter {
        ranked_records.retain(|record| filter_instance.matches(&record.device()));
    }

    let default_time = DateTime::<Utc>::from_timestamp(0, 0).unwrap();

    ranked_records.sort_by(|record_a, record_b| {
        let project_picked_a = record_a.project_last_picked_at.get(&project_path);
        let project_picked_b = record_b.project_last_picked_at.get(&project_path);

        match (project_picked_a, project_picked_b) {
            (Some(a), Some(b)) => return b.cmp(a),
            (Some(_), None) => return std::cmp::Ordering::Less,
            (None, Some(_)) => return std::cmp::Ordering::Greater,
            (None, None) => {}
        }

        let last_picked_a = record_a.last_picked_at.unwrap_or(default_time);
        let last_picked_b = record_b.last_picked_at.unwrap_or(default_time);
        let pick_comparison = last_picked_b.cmp(&last_picked_a);
        if pick_comparison != std::cmp::Ordering::Equal {
            return pick_comparison;
        }

        let last_seen_a = record_a.last_seen_at.unwrap_or(default_time);
        let last_seen_b = record_b.last_seen_at.unwrap_or(default_time);
        let seen_comparison = last_seen_b.cmp(&last_seen_a);
        if seen_comparison != std::cmp::Ordering::Equal {
            return seen_comparison;
        }

        record_a.name.cmp(&record_b.name)
    });

    let devices_for_prompt: Vec<FlutterDevice> =
        ranked_records.iter().map(DeviceRecord::device).collect();

    if devices_for_prompt.is_empty() {
        eprintln!("{}", red("No devices available."));
        eprintln!(
            "{}",
            gray("Run \"fl device refresh\" to update the device cache.")
        );
        std::process::exit(64);
    }

    if auto_yes {
        let selected = &devices_for_prompt[0];
        term_println(&gray(&format!(
            "Auto-selecting first device: {} ({})",
            selected.name, selected.id
        )));
        record_device_pick(&selected.id, Some(&project_path), verbose);
        return Some(selected.id.clone());
    }

    if !stdin().is_terminal() {
        eprintln!(
            "{}",
            red(
                "stdin is not a terminal; specify a device with -d <deviceId> or use -y to auto-select."
            )
        );
        std::process::exit(64);
    }

    let mut selection = DeviceSelectionContext::new();
    selection.initialize(&devices_for_prompt);
    print_device_choices(&selection);

    let mut session = SelectionSession::new();
    let selected_id = prompt_device_selection(
        &mut selection,
        flutter_command,
        filter.as_ref(),
        &mut session,
        using_cached_devices,
        verbose,
    )
    .await;
    session.deactivate();

    if let Some(ref device_id) = selected_id {
        record_device_pick(device_id, Some(&project_path), verbose);
    }

    selected_id
}

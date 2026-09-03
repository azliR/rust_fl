use std::io::{IsTerminal, stdin};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use notify::{EventKind, RecursiveMode, Watcher};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::common::ansi::{cyan, format_timestamp, gray, green, red, yellow};
use crate::devices::selector::resolve_device_id;
use crate::flutter::command::{FlutterCommand, resolve_flutter_command};
use crate::flutter::vm_service::start_vm_service_listener;

struct TerminalRawGuard;

impl TerminalRawGuard {
    fn enter() -> Option<Self> {
        if crossterm::terminal::enable_raw_mode().is_ok() {
            Some(Self)
        } else {
            None
        }
    }
}

impl Drop for TerminalRawGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Runs Flutter with enhanced logging, automatic file watcher reload, and device selection.
pub struct FlutterRunner {
    pub forwarded_args: Vec<String>,
    pub verbose: bool,
    pub platform_override: Option<String>,
    pub force_device_refresh: bool,
    pub auto_yes: bool,
    pub flutter_command: FlutterCommand,
}

impl FlutterRunner {
    pub fn new(
        forwarded_args: Vec<String>,
        platform_override: Option<String>,
        verbose: bool,
        force_device_refresh: bool,
        auto_yes: bool,
        flutter_command: Option<FlutterCommand>,
    ) -> Self {
        let command = flutter_command.unwrap_or_else(|| resolve_flutter_command(None));
        Self {
            forwarded_args,
            verbose,
            platform_override,
            force_device_refresh,
            auto_yes,
            flutter_command: command,
        }
    }

    /// Executes the full Flutter supervision lifecycle.
    pub async fn run(self) -> i32 {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            cyan("🚀 Starting Flutter with enhanced features...")
        );

        let device_id = resolve_device_id(
            &self.flutter_command,
            &self.forwarded_args,
            self.platform_override.as_deref(),
            self.force_device_refresh,
            self.auto_yes,
            self.verbose,
        )
        .await;

        let mut flutter_args = vec!["run".to_string()];
        if let Some(id) = device_id {
            flutter_args.push("-d".to_string());
            flutter_args.push(id);
        }
        flutter_args.extend_from_slice(&self.forwarded_args);

        let command_args = self.flutter_command.with_args(&flutter_args);
        if self.verbose {
            println!(
                "{} {}",
                gray(&format_timestamp()),
                gray(&format!(
                    "Running: {}",
                    self.flutter_command.describe(&command_args)
                ))
            );
        }

        let mut child = match tokio::process::Command::new(&self.flutter_command.executable)
            .args(&command_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                eprintln!(
                    "{}",
                    red(&format!("Failed to spawn Flutter process: {error}"))
                );
                return 1;
            }
        };

        let child_stdin = child.stdin.take().expect("Failed to capture child stdin");
        let child_stdout = child.stdout.take().expect("Failed to capture child stdout");
        let child_stderr = child.stderr.take().expect("Failed to capture child stderr");

        let app_started = Arc::new(AtomicBool::new(false));
        let is_reloading = Arc::new(AtomicBool::new(false));
        let vm_connected = Arc::new(AtomicBool::new(false));

        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(32);

        let mut stdin_writer = child_stdin;
        tokio::spawn(async move {
            while let Some(command) = stdin_rx.recv().await {
                if stdin_writer.write_all(command.as_bytes()).await.is_err() {
                    break;
                }
                if stdin_writer.flush().await.is_err() {
                    break;
                }
            }
        });

        let vm_service_regex =
            Regex::new(r"(?i)(?:VM\s+Service|Observatory|Dart\s+VM\s+Service).*?(http://[^\s]+)")
                .expect("Valid regex");

        let app_started_clone = Arc::clone(&app_started);
        let vm_connected_clone = Arc::clone(&vm_connected);
        let verbose = self.verbose;

        let vm_service_regex_stdout = vm_service_regex.clone();
        let vm_service_regex_stderr = vm_service_regex;

        tokio::spawn(async move {
            let mut reader = BufReader::new(child_stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                process_flutter_output(
                    &line,
                    &app_started_clone,
                    &vm_connected_clone,
                    &vm_service_regex_stdout,
                    verbose,
                );
            }
        });

        let app_started_clone2 = Arc::clone(&app_started);
        let vm_connected_clone2 = Arc::clone(&vm_connected);
        tokio::spawn(async move {
            let mut reader = BufReader::new(child_stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                process_flutter_output(
                    &line,
                    &app_started_clone2,
                    &vm_connected_clone2,
                    &vm_service_regex_stderr,
                    verbose,
                );
            }
        });

        let (reload_tx, mut reload_rx) = mpsc::channel::<()>(8);
        let (restart_tx, mut restart_rx) = mpsc::channel::<()>(8);

        let is_reloading_action = Arc::clone(&is_reloading);
        let stdin_tx_action = stdin_tx.clone();
        tokio::spawn(async move {
            while reload_rx.recv().await.is_some() {
                if is_reloading_action.swap(true, Ordering::SeqCst) {
                    continue;
                }
                println!("{} {}", gray(&format_timestamp()), cyan("🔥 Hot reload..."));
                let _ = stdin_tx_action.send("r".to_string()).await;
                tokio::time::sleep(Duration::from_millis(1000)).await;
                is_reloading_action.store(false, Ordering::SeqCst);
            }
        });

        let is_reloading_restart = Arc::clone(&is_reloading);
        let stdin_tx_restart = stdin_tx.clone();
        tokio::spawn(async move {
            while restart_rx.recv().await.is_some() {
                if is_reloading_restart.swap(true, Ordering::SeqCst) {
                    continue;
                }
                println!(
                    "{} {}",
                    gray(&format_timestamp()),
                    cyan("🔄 Hot restart...")
                );
                let _ = stdin_tx_restart.send("R".to_string()).await;
                tokio::time::sleep(Duration::from_millis(2000)).await;
                is_reloading_restart.store(false, Ordering::SeqCst);
            }
        });

        let has_terminal = stdin().is_terminal();
        if has_terminal {
            let app_started_kb = Arc::clone(&app_started);
            let reload_tx_kb = reload_tx.clone();
            let restart_tx_kb = restart_tx.clone();
            let stdin_tx_kb = stdin_tx.clone();

            std::thread::spawn(move || {
                let _guard = TerminalRawGuard::enter();
                loop {
                    let key = match event::read() {
                        Ok(Event::Key(k)) => k,
                        Ok(_) => continue,
                        Err(_) => break,
                    };

                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        println!("{}", cyan("\n👋 Quitting..."));
                        std::process::exit(130);
                    }

                    match key.code {
                        KeyCode::Char('r') => {
                            if !app_started_kb.load(Ordering::SeqCst) {
                                println!(
                                    "{} {}",
                                    gray(&format_timestamp()),
                                    yellow("⏳ Waiting for app to start...")
                                );
                                continue;
                            }
                            let _ = reload_tx_kb.blocking_send(());
                        }
                        KeyCode::Char('R') => {
                            if !app_started_kb.load(Ordering::SeqCst) {
                                println!(
                                    "{} {}",
                                    gray(&format_timestamp()),
                                    yellow("⏳ Waiting for app to start...")
                                );
                                continue;
                            }
                            let _ = restart_tx_kb.blocking_send(());
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            println!("{}", cyan("\n👋 Quitting..."));
                            let _ = stdin_tx_kb.blocking_send("q".to_string());
                            break;
                        }
                        KeyCode::Char('h') | KeyCode::Char('H') => {
                            show_interactive_help();
                        }
                        _ => {}
                    }
                }
            });
        }

        setup_file_watcher(Arc::clone(&app_started), reload_tx.clone());

        tokio::select! {
            result = child.wait() => {
                match result {
                    Ok(status) => status.code().unwrap_or(0),
                    Err(_) => 1,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("{}", cyan("\n👋 Received Ctrl+C; cleaning up..."));
                let _ = child.kill().await;
                130
            }
        }
    }
}

/// Formats and parses child Flutter output lines.
fn process_flutter_output(
    line: &str,
    app_started: &Arc<AtomicBool>,
    vm_connected: &Arc<AtomicBool>,
    vm_service_regex: &Regex,
    verbose: bool,
) {
    if line.trim().is_empty() {
        return;
    }

    println!("{} {line}", gray(&format_timestamp()));

    if let Some(captures) = vm_service_regex.captures(line)
        && let Some(uri_match) = captures.get(1)
    {
        let uri = uri_match.as_str().to_string();
        if !vm_connected.swap(true, Ordering::SeqCst) {
            if verbose {
                println!(
                    "{} {}",
                    gray(&format_timestamp()),
                    gray(&format!("Found VM Service URI: {uri}"))
                );
            }
            tokio::spawn(start_vm_service_listener(uri, verbose));
        }
    }

    if (line.contains("Flutter run key commands")
        || line.contains("An Observatory debugger")
        || line.contains("A Dart VM Service"))
        && !app_started.swap(true, Ordering::SeqCst)
    {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            green("✓ App started successfully")
        );
        println!(
            "{} {}",
            gray(&format_timestamp()),
            cyan("Commands: r=reload, R=restart, q=quit, h=help")
        );
    }

    if line.contains("Reloaded") || line.contains("reloaded") {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            green("✓ Hot reload complete")
        );
    }

    if line.contains("Restarted") || line.contains("restarted") {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            green("✓ Hot restart complete")
        );
    }
}

/// Watches the `lib/` directory and triggers debounced hot reloads on Dart file modifications.
fn setup_file_watcher(app_started: Arc<AtomicBool>, reload_tx: mpsc::Sender<()>) {
    let lib_dir = Path::new("lib");
    if !lib_dir.is_dir() {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            yellow("Warning: lib directory not found")
        );
        return;
    }

    println!(
        "{} {}",
        gray(&format_timestamp()),
        gray("👀 Watching for file changes in lib/...")
    );

    let (file_event_tx, mut file_event_rx) = mpsc::channel::<PathBuf>(32);

    let mut watcher = match notify::recommended_watcher(move |res: Result<notify::Event, _>| {
        if let Ok(event) = res
            && matches!(event.kind, EventKind::Modify(_))
        {
            for path in event.paths {
                if path.extension().and_then(|ext| ext.to_str()) == Some("dart") {
                    let _ = file_event_tx.blocking_send(path);
                }
            }
        }
    }) {
        Ok(w) => w,
        Err(_) => return,
    };

    if watcher.watch(lib_dir, RecursiveMode::Recursive).is_err() {
        return;
    }

    std::mem::forget(watcher);

    tokio::spawn(async move {
        let mut debounce_timer: Option<tokio::time::Instant> = None;
        let mut pending_file_name = String::new();

        loop {
            tokio::select! {
                Some(path) = file_event_rx.recv() => {
                    if !app_started.load(Ordering::SeqCst) {
                        continue;
                    }
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        pending_file_name = file_name.to_string();
                    }
                    debounce_timer = Some(tokio::time::Instant::now() + Duration::from_millis(500));
                }
                _ = async {
                    match debounce_timer {
                        Some(deadline) => tokio::time::sleep_until(deadline).await,
                        None => std::future::pending().await,
                    }
                } => {
                    debounce_timer = None;
                    if app_started.load(Ordering::SeqCst) {
                        println!(
                            "{} {}",
                            gray(&format_timestamp()),
                            cyan(&format!("📝 File changed: {pending_file_name}"))
                        );
                        let _ = reload_tx.send(()).await;
                    }
                }
            }
        }
    });
}

/// Displays interactive keyboard help.
fn show_interactive_help() {
    println!();
    println!("{}", cyan("═══════════════════════════════"));
    println!("{}", cyan("  Available Commands"));
    println!("{}", cyan("═══════════════════════════════"));
    println!("  {} - Hot reload (fast refresh)", cyan("r"));
    println!("  {} - Hot restart (full restart)", cyan("R"));
    println!("  {} - Quit application", cyan("q"));
    println!("  {} - Show this help", cyan("h"));
    println!("{}", cyan("═══════════════════════════════"));
    println!();
}

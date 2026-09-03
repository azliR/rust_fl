use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::common::ansi::{format_timestamp, gray, green, red, yellow};
use crate::common::terminal::term_println;
use crate::flutter::log_filter::LogFilter;

/// Connects to Dart VM Service via WebSocket and listens for Stdout, Stderr, and Logging events.
pub async fn start_vm_service_listener(raw_uri: String, log_filter: Arc<LogFilter>, verbose: bool) {
    let ws_url = normalize_vm_service_url(&raw_uri);
    if verbose {
        term_println(&format!(
            "{} {}",
            gray(&format_timestamp()),
            gray(&format!("Connecting to VM Service at {ws_url}"))
        ));
    }

    let connection_result = connect_async(&ws_url).await;
    let (mut ws_stream, _) = match connection_result {
        Ok(stream) => stream,
        Err(error) => {
            let fallback_url = if ws_url.ends_with("/ws") {
                ws_url.trim_end_matches("/ws").to_string()
            } else {
                format!("{}/ws", ws_url.trim_end_matches('/'))
            };

            match connect_async(&fallback_url).await {
                Ok(stream) => stream,
                Err(_) => {
                    term_println(&format!(
                        "{} {}",
                        gray(&format_timestamp()),
                        red(&format!("Failed to connect to VM Service: {error}"))
                    ));
                    if verbose {
                        term_println(&format!(
                            "{} {}",
                            gray(&format_timestamp()),
                            gray("Enhanced logging will not be available")
                        ));
                    }
                    return;
                }
            }
        }
    };

    term_println(&format!(
        "{} {}",
        gray(&format_timestamp()),
        green("✓ Connected to VM Service for enhanced logging")
    ));

    let streams_to_listen = ["Stdout", "Stderr", "Logging"];
    for (id_index, stream_id) in streams_to_listen.iter().enumerate() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "streamListen",
            "params": {
                "streamId": stream_id
            },
            "id": id_index + 1
        });

        let json_string = request.to_string();
        if let Err(error) = ws_stream.send(Message::Text(json_string.into())).await
            && verbose
        {
            term_println(&format!(
                "{} {}",
                gray(&format_timestamp()),
                gray(&format!("Failed to subscribe to {stream_id}: {error}"))
            ));
        }
    }

    while let Some(msg_result) = ws_stream.next().await {
        let msg = match msg_result {
            Ok(Message::Text(text)) => text.to_string(),
            Ok(Message::Binary(bin)) => String::from_utf8_lossy(&bin).to_string(),
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => continue,
        };

        handle_vm_event(&msg, &log_filter, verbose);
    }
}

/// Normalizes Dart VM Service HTTP URL to WebSocket ws:// URL.
pub fn normalize_vm_service_url(url_string: &str) -> String {
    let trimmed = url_string.trim();
    let mut ws_url = if let Some(stripped) = trimmed.strip_prefix("http://") {
        format!("ws://{stripped}")
    } else if let Some(stripped) = trimmed.strip_prefix("https://") {
        format!("wss://{stripped}")
    } else if !trimmed.starts_with("ws://") && !trimmed.starts_with("wss://") {
        format!("ws://{trimmed}")
    } else {
        trimmed.to_string()
    };

    if !ws_url.ends_with("/ws") {
        if ws_url.ends_with('/') {
            ws_url.push_str("ws");
        } else {
            ws_url.push_str("/ws");
        }
    }

    ws_url
}

/// Dispatches stream events for Stdout, Stderr, and Logging.
fn handle_vm_event(json_text: &str, log_filter: &LogFilter, verbose: bool) {
    let Ok(data) = serde_json::from_str::<Value>(json_text) else {
        return;
    };

    let Some(method) = data.get("method").and_then(Value::as_str) else {
        return;
    };
    if method != "streamNotify" {
        return;
    }

    let Some(params) = data.get("params") else {
        return;
    };
    let stream_id = params.get("streamId").and_then(Value::as_str).unwrap_or("");
    let Some(event) = params.get("event") else {
        return;
    };

    if stream_id == "Stdout" {
        if let Some(bytes_b64) = event.get("bytes").and_then(Value::as_str) {
            match BASE64.decode(bytes_b64) {
                Ok(bytes) => {
                    let decoded = String::from_utf8_lossy(&bytes);
                    let trimmed = decoded.trim_end();
                    if !trimmed.is_empty() && !log_filter.should_ignore(trimmed) {
                        term_println(&format!("{} {trimmed}", gray(&format_timestamp())));
                    }
                }
                Err(error) => {
                    if verbose {
                        term_println(&format!(
                            "{} {}",
                            gray(&format_timestamp()),
                            gray(&format!("Failed to decode stdout: {error}"))
                        ));
                    }
                }
            }
        }
        return;
    }

    if stream_id == "Stderr" {
        if let Some(bytes_b64) = event.get("bytes").and_then(Value::as_str) {
            match BASE64.decode(bytes_b64) {
                Ok(bytes) => {
                    let decoded = String::from_utf8_lossy(&bytes);
                    let trimmed = decoded.trim_end();
                    if !trimmed.is_empty() && !log_filter.should_ignore(trimmed) {
                        term_println(&format!("{} {}", gray(&format_timestamp()), red(trimmed)));
                    }
                }
                Err(error) => {
                    if verbose {
                        term_println(&format!(
                            "{} {}",
                            gray(&format_timestamp()),
                            gray(&format!("Failed to decode stderr: {error}"))
                        ));
                    }
                }
            }
        }
        return;
    }

    if stream_id == "Logging"
        && let Some(record) = event.get("logRecord")
    {
        let logger_name = extract_instance_string(record.get("loggerName"));
        let log_message = extract_instance_string(record.get("message"));
        let error_string = extract_instance_string(record.get("error"));
        let stack_trace = extract_instance_string(record.get("stackTrace"));
        let level = record
            .get("level")
            .map(|l| l.to_string())
            .unwrap_or_default();

        let prefix = if !logger_name.is_empty() {
            format!("[{logger_name}]")
        } else if !level.is_empty() && level != "-1" {
            format!("[L{level}]")
        } else {
            String::new()
        };

        let mut builder = format!("📝 {prefix} {log_message}");
        if !error_string.is_empty() {
            builder.push_str(&format!("  error: {error_string}"));
        }
        if !stack_trace.is_empty() {
            builder.push('\n');
            builder.push_str(&stack_trace);
        }

        if !log_filter.should_ignore(&builder) {
            term_println(&format!(
                "{} {}",
                gray(&format_timestamp()),
                yellow(&builder)
            ));
        }
    }
}

/// Helper to extract string value from an InstanceRef or string value in JSON.
fn extract_instance_string(value: Option<&Value>) -> String {
    let Some(val) = value else {
        return String::new();
    };

    if let Some(s) = val.as_str() {
        return s.to_string();
    }

    if let Some(val_str) = val.get("valueAsString").and_then(Value::as_str) {
        return val_str.to_string();
    }

    String::new()
}

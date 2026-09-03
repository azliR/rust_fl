use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::common::ansi::{format_timestamp, gray, green, red, yellow};

/// Connects to Dart VM Service via WebSocket and listens for Stdout, Stderr, and Logging events.
pub async fn start_vm_service_listener(raw_uri: String, verbose: bool) {
    let ws_url = normalize_vm_service_url(&raw_uri);
    if verbose {
        println!(
            "{} {}",
            gray(&format_timestamp()),
            gray(&format!("Connecting to VM Service at {ws_url}"))
        );
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
                    println!(
                        "{} {}",
                        gray(&format_timestamp()),
                        red(&format!("Failed to connect to VM Service: {error}"))
                    );
                    if verbose {
                        println!(
                            "{} {}",
                            gray(&format_timestamp()),
                            gray("Enhanced logging will not be available")
                        );
                    }
                    return;
                }
            }
        }
    };

    println!(
        "{} {}",
        gray(&format_timestamp()),
        green("✓ Connected to VM Service for enhanced logging")
    );

    let streams_to_listen = ["Stdout", "Stderr", "Logging"];
    for (id_index, stream_id) in streams_to_listen.iter().enumerate() {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "streamListen",
            "params": {
                "streamId": stream_id
            },
            "id": (id_index + 1).to_string()
        });

        if ws_stream
            .send(Message::Text(request.to_string().into()))
            .await
            .is_err()
        {
            return;
        }
    }

    while let Some(msg_result) = ws_stream.next().await {
        let message = match msg_result {
            Ok(Message::Text(text)) => text,
            Ok(_) => continue,
            Err(_) => break,
        };

        handle_vm_message(&message, verbose);
    }
}

/// Normalizes HTTP/HTTPS VM service URIs into WebSocket URIs.
pub fn normalize_vm_service_url(raw_uri: &str) -> String {
    let mut ws_uri = raw_uri
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    if ws_uri.ends_with("/ws") || ws_uri.ends_with("/ws/") {
        return ws_uri;
    }

    if ws_uri.ends_with('/') {
        ws_uri.push_str("ws");
    } else {
        ws_uri.push_str("/ws");
    }

    ws_uri
}

/// Processes an incoming JSON-RPC event message from the Dart VM Service.
fn handle_vm_message(message: &str, verbose: bool) {
    let parsed: Value = match serde_json::from_str(message) {
        Ok(value) => value,
        Err(_) => return,
    };

    if parsed.get("method").and_then(Value::as_str) != Some("streamNotify") {
        return;
    }

    let Some(params) = parsed.get("params") else {
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
                    if !trimmed.is_empty() {
                        println!("{} {trimmed}", gray(&format_timestamp()));
                    }
                }
                Err(error) => {
                    if verbose {
                        println!(
                            "{} {}",
                            gray(&format_timestamp()),
                            gray(&format!("Failed to decode stdout: {error}"))
                        );
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
                    if !trimmed.is_empty() {
                        println!("{} {}", gray(&format_timestamp()), red(trimmed));
                    }
                }
                Err(error) => {
                    if verbose {
                        println!(
                            "{} {}",
                            gray(&format_timestamp()),
                            gray(&format!("Failed to decode stderr: {error}"))
                        );
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

        println!("{} {}", gray(&format_timestamp()), yellow(&builder));
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

    if let Some(obj) = val.as_object() {
        if let Some(kind) = obj.get("kind").and_then(Value::as_str)
            && kind == "Null"
        {
            return String::new();
        }
        if let Some(string_val) = obj.get("valueAsString").and_then(Value::as_str) {
            if string_val == "null" {
                return String::new();
            }
            return string_val.to_string();
        }
    }

    String::new()
}

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, oneshot};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::common::ansi::{format_timestamp, gray, green, red, yellow};
use crate::common::terminal::term_println;
use crate::flutter::log_filter::LogFilter;

type WsWrite = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>;

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
    let ws_stream = match connection_result {
        Ok((stream, _)) => stream,
        Err(error) => {
            let fallback_url = if ws_url.ends_with("/ws") {
                ws_url.trim_end_matches("/ws").to_string()
            } else {
                format!("{}/ws", ws_url.trim_end_matches('/'))
            };

            match connect_async(&fallback_url).await {
                Ok((stream, _)) => stream,
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

    let (write_half, mut read_half) = ws_stream.split();
    let write: Arc<Mutex<WsWrite>> = Arc::new(Mutex::new(write_half));
    let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
    let id_counter = Arc::new(AtomicU64::new(100));

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
        let send_error = {
            let mut guard = write.lock().await;
            guard.send(Message::Text(json_string.into())).await.err()
        };
        if let Some(error) = send_error
            && verbose
        {
            term_println(&format!(
                "{} {}",
                gray(&format_timestamp()),
                gray(&format!("Failed to subscribe to {stream_id}: {error}"))
            ));
        }
    }

    while let Some(msg_result) = read_half.next().await {
        let msg = match msg_result {
            Ok(Message::Text(text)) => text.to_string(),
            Ok(Message::Binary(bin)) => String::from_utf8_lossy(&bin).to_string(),
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => continue,
        };

        let Ok(data) = serde_json::from_str::<Value>(&msg) else {
            continue;
        };

        if data.get("id").is_some() && data.get("method").is_none() {
            route_rpc_response(&data, &pending).await;
            continue;
        }

        handle_vm_event(&data, &log_filter, verbose, &write, &pending, &id_counter).await;
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

/// Routes a JSON-RPC response to the pending getObject waiter matching its id.
async fn route_rpc_response(data: &Value, pending: &PendingMap) {
    let Some(id_key) = data.get("id").and_then(|id| match id {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }) else {
        return;
    };
    let sender = pending.lock().await.remove(&id_key);
    if let Some(tx) = sender {
        let result = data.get("result").cloned().unwrap_or(Value::Null);
        let _ = tx.send(result);
    }
}

/// Dispatches stream events for Stdout, Stderr, and Logging.
async fn handle_vm_event(
    data: &Value,
    log_filter: &Arc<LogFilter>,
    verbose: bool,
    write: &Arc<Mutex<WsWrite>>,
    pending: &PendingMap,
    id_counter: &Arc<AtomicU64>,
) {
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
                    for line in decoded.split(['\r', '\n']) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && !log_filter.should_ignore(trimmed) {
                            term_println(&format!("{} {trimmed}", gray(&format_timestamp())));
                        }
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
                    for line in decoded.split(['\r', '\n']) {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && !log_filter.should_ignore(trimmed) {
                            term_println(&format!(
                                "{} {}",
                                gray(&format_timestamp()),
                                red(trimmed)
                            ));
                        }
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
        let isolate_id = isolate_id_from_event(event);
        let needs_full_fetch = is_value_truncated(record.get("message"))
            || is_value_truncated(record.get("error"))
            || is_value_truncated(record.get("stackTrace"))
            || is_value_truncated(record.get("loggerName"));

        if !needs_full_fetch {
            print_logging_record(
                &extract_instance_string(record.get("loggerName")),
                &extract_instance_string(record.get("message")),
                &extract_instance_string(record.get("error")),
                &extract_instance_string(record.get("stackTrace")),
                record
                    .get("level")
                    .map(|l| l.to_string())
                    .unwrap_or_default(),
                log_filter,
            );
            return;
        }

        let Some(isolate) = isolate_id else {
            print_logging_record(
                &extract_instance_string(record.get("loggerName")),
                &extract_instance_string(record.get("message")),
                &extract_instance_string(record.get("error")),
                &extract_instance_string(record.get("stackTrace")),
                record
                    .get("level")
                    .map(|l| l.to_string())
                    .unwrap_or_default(),
                log_filter,
            );
            return;
        };

        let record_owned = record.clone();
        let log_filter_clone = Arc::clone(log_filter);
        let write_clone = Arc::clone(write);
        let pending_clone = Arc::clone(pending);
        let counter_clone = Arc::clone(id_counter);
        tokio::spawn(async move {
            let logger_name = resolve_instance_string(
                &write_clone,
                &pending_clone,
                &counter_clone,
                &isolate,
                record_owned.get("loggerName"),
            )
            .await;
            let log_message = resolve_instance_string(
                &write_clone,
                &pending_clone,
                &counter_clone,
                &isolate,
                record_owned.get("message"),
            )
            .await;
            let error_string = resolve_instance_string(
                &write_clone,
                &pending_clone,
                &counter_clone,
                &isolate,
                record_owned.get("error"),
            )
            .await;
            let stack_trace = resolve_instance_string(
                &write_clone,
                &pending_clone,
                &counter_clone,
                &isolate,
                record_owned.get("stackTrace"),
            )
            .await;
            let level = record_owned
                .get("level")
                .map(|l| l.to_string())
                .unwrap_or_default();
            print_logging_record(
                &logger_name,
                &log_message,
                &error_string,
                &stack_trace,
                level,
                &log_filter_clone,
            );
        });
    }
}

/// Prints a Logging stream record with timestamp and yellow styling.
fn print_logging_record(
    logger_name: &str,
    log_message: &str,
    error_string: &str,
    stack_trace: &str,
    level: String,
    log_filter: &LogFilter,
) {
    let prefix = if !logger_name.is_empty() && !level.is_empty() && level != "0" && level != "-1" {
        format!("[{logger_name}:L{level}] ")
    } else if !logger_name.is_empty() {
        format!("[{logger_name}] ")
    } else if !level.is_empty() && level != "0" && level != "-1" {
        format!("[L{level}] ")
    } else {
        String::new()
    };

    let mut builder = format!("📝 {prefix}{log_message}");
    if !error_string.is_empty() {
        builder.push_str(&format!("  error: {error_string}"));
    }
    if !stack_trace.is_empty() {
        builder.push('\n');
        builder.push_str(stack_trace);
    }

    if !log_filter.should_ignore(&builder) {
        term_println(&format!(
            "{} {}",
            gray(&format_timestamp()),
            yellow(&builder)
        ));
    }
}

/// Returns the isolate id associated with a Logging event.
fn isolate_id_from_event(event: &Value) -> Option<String> {
    event
        .get("isolate")
        .and_then(|iso| iso.get("id"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// Reports whether an InstanceRef was truncated by the VM.
fn is_value_truncated(value: Option<&Value>) -> bool {
    value
        .and_then(|v| v.get("valueAsStringIsTruncated"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Returns the object id of an InstanceRef when present.
fn instance_object_id(value: Option<&Value>) -> Option<String> {
    value
        .and_then(|v| v.get("id"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}

/// Resolves an InstanceRef to its full string, fetching via getObject when truncated.
async fn resolve_instance_string(
    write: &Arc<Mutex<WsWrite>>,
    pending: &PendingMap,
    id_counter: &Arc<AtomicU64>,
    isolate_id: &str,
    raw: Option<&Value>,
) -> String {
    let fallback = extract_instance_string(raw);
    if !is_value_truncated(raw) {
        return fallback;
    }
    let Some(object_id) = instance_object_id(raw) else {
        return fallback;
    };
    let full = fetch_full_string(write, pending, id_counter, isolate_id, &object_id).await;
    if full.is_empty() { fallback } else { full }
}

/// Fetches the full string value of an object via getObject, paging long strings.
async fn fetch_full_string(
    write: &Arc<Mutex<WsWrite>>,
    pending: &PendingMap,
    id_counter: &Arc<AtomicU64>,
    isolate_id: &str,
    object_id: &str,
) -> String {
    let Some(first) = send_get_object(
        write, pending, id_counter, isolate_id, object_id, None, None,
    )
    .await
    else {
        return String::new();
    };
    if let Some(kind) = first.get("type").and_then(Value::as_str)
        && kind.contains("Sentinel")
    {
        return String::new();
    }
    let Some(first_chunk) = first.get("valueAsString").and_then(Value::as_str) else {
        return String::new();
    };
    let mut full = first_chunk.to_string();
    let length = first.get("length").and_then(Value::as_u64);
    let count = first.get("count").and_then(Value::as_u64);
    let start_offset = first.get("offset").and_then(Value::as_u64).unwrap_or(0);
    let (Some(total), Some(done)) = (length, count) else {
        return full;
    };
    let mut offset = start_offset.saturating_add(done);
    let mut guard = 0;
    while offset < total && guard < 100 {
        guard += 1;
        let want = (total - offset).min(10_000);
        let Some(next) = send_get_object(
            write,
            pending,
            id_counter,
            isolate_id,
            object_id,
            Some(offset),
            Some(want),
        )
        .await
        else {
            break;
        };
        let Some(chunk) = next.get("valueAsString").and_then(Value::as_str) else {
            break;
        };
        full.push_str(chunk);
        let advanced = next
            .get("count")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| chunk.chars().count() as u64);
        if advanced == 0 {
            break;
        }
        offset = offset.saturating_add(advanced);
        if full.len() > 1_000_000 {
            break;
        }
    }
    full
}

/// Sends a single getObject request and waits for the matching response.
async fn send_get_object(
    write: &Arc<Mutex<WsWrite>>,
    pending: &PendingMap,
    id_counter: &Arc<AtomicU64>,
    isolate_id: &str,
    object_id: &str,
    offset: Option<u64>,
    count: Option<u64>,
) -> Option<Value> {
    let request_id = format!("fl-{}", id_counter.fetch_add(1, Ordering::SeqCst));
    let (tx, rx) = oneshot::channel();
    pending.lock().await.insert(request_id.clone(), tx);
    let mut params = serde_json::Map::new();
    params.insert(
        "isolateId".to_string(),
        Value::String(isolate_id.to_string()),
    );
    params.insert("objectId".to_string(), Value::String(object_id.to_string()));
    if let Some(value) = offset {
        params.insert("offset".to_string(), json!(value));
    }
    if let Some(value) = count {
        params.insert("count".to_string(), json!(value));
    }
    let request = json!({
        "jsonrpc": "2.0",
        "method": "getObject",
        "params": Value::Object(params),
        "id": request_id.clone(),
    });
    let send_ok = {
        let mut guard = write.lock().await;
        guard
            .send(Message::Text(request.to_string().into()))
            .await
            .is_ok()
    };
    if !send_ok {
        pending.lock().await.remove(&request_id);
        return None;
    }
    match tokio::time::timeout(Duration::from_secs(5), rx).await {
        Ok(Ok(result)) => Some(result),
        _ => {
            pending.lock().await.remove(&request_id);
            None
        }
    }
}

/// Helper to extract string value from an InstanceRef or string value in JSON.
fn extract_instance_string(value: Option<&Value>) -> String {
    let Some(val) = value else {
        return String::new();
    };

    if val.is_null() {
        return String::new();
    }

    if let Some(kind) = val.get("kind").and_then(Value::as_str)
        && kind == "Null"
    {
        return String::new();
    }

    if let Some(s) = val.as_str() {
        if s == "null" {
            return String::new();
        }
        return s.to_string();
    }

    if let Some(val_str) = val.get("valueAsString").and_then(Value::as_str) {
        if val_str == "null" {
            return String::new();
        }
        return val_str.to_string();
    }

    String::new()
}

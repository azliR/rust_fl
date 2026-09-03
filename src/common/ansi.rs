use chrono::Local;

/// Formats text with ANSI cyan color.
pub fn cyan(text: &str) -> String {
    format!("\x1B[36m{text}\x1B[0m")
}

/// Formats text with ANSI green color.
pub fn green(text: &str) -> String {
    format!("\x1B[32m{text}\x1B[0m")
}

/// Formats text with ANSI yellow color.
pub fn yellow(text: &str) -> String {
    format!("\x1B[33m{text}\x1B[0m")
}

/// Formats text with ANSI red color.
pub fn red(text: &str) -> String {
    format!("\x1B[31m{text}\x1B[0m")
}

/// Formats text with ANSI gray color.
pub fn gray(text: &str) -> String {
    format!("\x1B[90m{text}\x1B[0m")
}

/// Formats the current local time as YYYY/MM/DD HH:MM:SS.
pub fn format_timestamp() -> String {
    Local::now().format("%Y/%m/%d %H:%M:%S").to_string()
}

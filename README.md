# fl 🚀

A high-performance, native Rust companion for `flutter run` that provides auto hot-reload, `dart:developer` logs in your terminal, and smart device management that remembers your favorites.

## ✨ Features

### 🔥 Auto Reload & Logging

- Watches `lib/` and hot reloads automatically when you save
- Shows `dart:developer` logs right in your terminal via Dart VM Service WebSocket
- All `flutter run` options work — just pass them through!
- Same `r`, `R`, `q` shortcuts you're used to

### 📱 Smart Device Management

- **Smart ranking** — Devices used in this project appear first
- **Recently used** — Sorted by most recent usage
- **Remembers everything** — Devices stick around across refreshes
- **Self-cleaning** — Unused devices auto-remove after 30 days
- **Smart filtering** — Only shows devices matching your project

## 📦 Installation

```sh
# Clone it
git clone https://github.com/azliR/rust_fl.git
cd fl_rust

# Build and install to ~/.cargo/bin
cargo install --path .
```

> 💡 Make sure `$HOME/.cargo/bin` is in your `PATH`!

## 🎮 Usage

### Running Your App

```sh
# Just run it — pick a device
fl run

# Skip the picker, grab the first one
fl run -y

# Specify a device directly
fl run -d iPhone

# With flavor and target
fl run --flavor staging --target lib/main_dev.dart

# Only iOS devices please
fl run --platform ios

# Hide noisy system or driver logs
fl run --filter-out "gralloc4"
fl run --filter-out "Empty SMPTE 2094-40 data"
```

### 🎯 Device Selection

When picking a device:

- **1-9** — Select by number
- **Enter** — Grab the first one
- **r** — Refresh the list
- **q** — Quit

### 🔧 Device Management

```sh
fl device list      # See what's cached
fl device refresh   # Update the list
fl device rm <id>   # Remove one
```

### 📋 Other Goodies

```sh
fl pub sort         # Alphabetize your pubspec deps
fl pub diagnose     # Diagnose PubGrub dependency solver deadlocks
fl pub diagnose --upgrade # Check major version upgrade conflicts
fl flutter doctor   # Pass commands to Flutter
```

### ⚙️ Run Options

| Option                   | What it does                               |
| ------------------------ | ------------------------------------------ |
| `-d <id>`                | Pick a device                              |
| `-y`                     | Auto-select first device                   |
| `--platform <name>`      | Filter by platform                         |
| `--force-device-refresh` | Force a fresh list                         |
| `--filter-out <pattern>` | Hide output matching pattern or regex      |

## ⌨️ Keyboard Shortcuts

| Key | Action      |
| --- | ----------- |
| `r` | Hot reload  |
| `R` | Hot restart |
| `q` | Quit        |
| `h` | Help        |

## 🔧 Troubleshooting

- **Flutter not found?** Make sure it's in your `PATH` or configured with FVM
- **Missing a device?** Run `fl device refresh` or press `r` during selection
- **Only watches `lib/`** — Use symlinks for code elsewhere, or just press `r`

---

Made with 🦀 by [@azliR](https://github.com/azliR)

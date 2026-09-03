# AGENTS.md

This document provides architectural guidance, conventions, and operational guidelines for agents working on `fl_rust`.

## Project Overview

`fl` is a high-performance native Rust Flutter CLI companion that provides automatic hot reloading via file watcher, `dart:developer` log capture via Dart VM Service WebSocket, smart project-aware device ranking and caching, and utility commands like `pub sort` and `pub diagnose`.

## Architecture & Code Structure

The project follows a standard modular Rust binary + library structure:

- `src/main.rs`: Minimal CLI executable entrypoint delegating to argument parsing and command handlers.
- `src/lib.rs`: Public library crate exporting internal modules for testing and reuse.
- `src/common/`:
  - `ansi.rs`: Terminal ANSI styling (`cyan`, `green`, `yellow`, `red`, `gray`) and local timestamp formatting (`YYYY/MM/DD HH:MM:SS`).
  - `constants.rs`: Global CLI constants such as `CLI_VERSION`, 30-day device staleness threshold, and platform directory mapping.
- `src/cli/`:
  - `args.rs`: Command line parsing, global options validation, and `fl run` argument separation.
  - `usage.rs`: Formatted manual screen and help text.
- `src/devices/`:
  - `models.rs`: `FlutterDevice`, `DeviceRecord`, and `DeviceSelectionChanges`.
  - `cache.rs`: JSON device cache file persistence (`device-cache.json`), 30-day stale pruning, and Flutter CLI machine parsing.
  - `filter.rs`: Platform directory detection (`android/`, `ios/`, etc.) and `--platform` filter enforcement.
  - `selector.rs`: Interactive single-key terminal prompt (`crossterm` raw mode), slot management, and 4-tier recency ranking.
  - `command.rs`: Command handler for `fl device` (`list`, `refresh`, `rm`).
- `src/flutter/`:
  - `command.rs`: FVM vs global Flutter detection and passthrough execution.
  - `runner.rs`: Flutter run supervisor handling process lifecycle, stdout/stderr timestamps, file watcher debounce, and interactive keyboard shortcuts.
  - `vm_service.rs`: Asynchronous Dart VM Service WebSocket client subscribing to `Stdout`, `Stderr`, and `Logging` streams via JSON-RPC 2.0.
- `src/pub_utils/`:
  - `sorter.rs`: Alphabetical dependency sorter for `pubspec.yaml` preserving structure, comments, and multi-line definitions.
  - `solver_parser.rs`: Parser for PubGrub failure trees, SDK pins, and version deadlocks.
  - `diagnoser.rs`: Dry-run execution engine and structured conflict report generator.
  - `command.rs`: Router for `fl pub` (`sort`, `diagnose`).
- `tests/`: Integration test suites covering arguments, device caching, filtering, pubspec sorting, and PubGrub parsing.

## Code Style & Conventions

- **No Comments in Code**: Do not add comments to the code, except for doc comments (`///` or `//!`) or critical warnings.
- **Early Returns**: Use early returns and guard clauses to minimize nesting.
- **Naming**: Use clear, descriptive names; avoid abbreviations.
- **Formatting**: Run `cargo fmt` after modifying any Rust files.
- **Linting**: Run `cargo clippy --all-targets -- -D warnings` to ensure zero warnings and errors.
- **Testing**: Run `cargo test` to ensure all tests pass.
- **Git Operations**: Use `smart-commit` and run `code-review` before finalizing significant changes.

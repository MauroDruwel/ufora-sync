# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-09-28

### Added
- Background OneDrive-like sync daemon with configurable interval.
- System taskbar / menu bar tray icon (`pystray`) with live sync status.
- Course selection management with individual sync toggles.
- Cross-platform support for macOS, Linux, and Windows.
- Automatic conflict detection with `duplicate` (`_edited`), `skip`, or `overwrite` strategies.
- Persistent configuration management in standard OS app data directories.
- CLI commands: `service` (headless daemon), `gui` (settings window), `sync` (instant one-off).

### Fixed
- Fixed Tkinter `TclError` during course tree switching.
- Fixed Brightspace course enrollment JSON parsing for UGent API structure.
- Fixed direct CLI invocation to preserve UGent authentication and filtering.

## [0.1.0] - 2026-09-24

### Added
- Initial release of `ufora-sync` with CustomTkinter GUI.
- SHA-256 local manifest engine to prevent overwriting modified files.

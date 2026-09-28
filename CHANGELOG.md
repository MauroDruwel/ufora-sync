# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-09-28

### Added
- **OneDrive-style sync daemon**: Continuous background sync with configurable interval.
- **Native system tray app**: Menu bar / tray icon (`pystray`) with dynamic status icons (Idle, Syncing, Up-to-date, Needs Login).
- **CustomTkinter GUI**: Course selection hub with individual toggles, log monitor, and folder launchers.
- **Fast incremental sync**: Brightspace TOC `LastModifiedDate` tracking to skip unchanged files without downloading.
- **Robust staging & atomic writes**: Downloads staged in OS temporary directory and installed atomically; interrupted syncs never leave stray folders.
- **Automatic legacy cleanup**: Sweeps and deletes leftover `_tmp_*` folders from older versions.
- **Cross-process lock**: File-based `ProcessLock` preventing race conditions between tray, GUI, and CLI sync passes.
- **Preserved module hierarchies**: Course files are organized into their exact module and submodule folder paths.
- **Smart conflict resolution**: Preserves student edits via `duplicate` (`<filename>_edited.<ext>`), `skip`, or `overwrite`.
- **Filtered non-downloadables**: Gracefully handles and skips non-file topics (e.g. Wooclap, YouTube links).
- **Mauro Quality Gate (MQG) compliance**: 100% audit score with automated CI pipelines and pull request templates.

### Fixed
- Fixed Tkinter `TclError` during rapid course tree switching.
- Fixed Brightspace course enrollment JSON parsing for UGent API structure.
- Fixed AppKit main-thread crash on macOS by decoupling tray and settings windows.

## [0.1.0] - 2026-09-24

### Added
- Initial release of `ufora-sync` with CustomTkinter GUI.
- SHA-256 local manifest engine to prevent overwriting modified files.

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.7] - 2026-10-30

### Fixed
- **Fully Native Token Renewal**:
  - Replaced the runtime `ufora refresh` subprocess bridge with an app-owned hidden WebView refresh flow, removing the external Python, `ufora-ai`, and Playwright dependency.
  - Persisted only the UGent and Microsoft sign-in-chain cookies (including HTTP-only session cookies) in a private `~/.d2l/session.json` file, restored them before background renewal, and disabled hidden-WebView throttling where supported.
  - Prevented stale near-expiry tokens from being recaptured during renewal and serialized concurrent refresh attempts.
  - Explicit logout now clears the token, saved browser session, and in-app authentication cookies.
  - Settings now forces a fresh hidden token renewal instead of reopening the interactive Ufora page, reports progress, and closes its temporary WebView after completion.
  - Limited close-to-tray behavior to the main window so authentication and refresh windows are destroyed normally.

## [1.0.6] - 2026-10-30

### Fixed
- **Eliminated Screen Flickering & Redirect Loops**:
  - Resolved infinite redirect loop between `ufora.ugent.be/d2l/login` and `elosp.ugent.be` by strictly selecting in-page login links and removing disruptive programmatic location overwrites.
  - Eliminated rapid-fire account tile click spam on Microsoft Entra ID with persistent single-execution session guards (`__ufora_tile_clicked` and `__ufora_kmsi_clicked`).
  - Scoped network request interception strictly to `ufora.ugent.be`, preventing prototype pollution or JavaScript exceptions from breaking Microsoft's single-page login application.
  - Removed duplicate script execution on WebKit page load finished events.
- **Headless Silent Background Token Auto-Renewal**:
  - Integrated system `ufora refresh` bridge into Rust engine to silently renew expiring Brightspace tokens in the background without opening any visible browser window.
  - Background sync passes no longer stall for 53 seconds on auth expiration or unexpectedly pop open visible windows during scheduled syncs.
- **Unified Sync Action**:
  - Synchronized both "Sync Now" buttons (App Header and macOS Tray Menu) to execute the exact same auth-check, silent-renewal, and fallback sign-in workflow.

## [1.0.5] - 2026-10-30

### Fixed
- **Synchronized Sync & Re-Authentication Actions**:
  - Unified the desktop window and menu bar tray "Sync Now" buttons so both reliably detect expired sessions and seamlessly launch the SSO re-authentication flow instead of silently failing with unhandled auth errors.
  - Clicking "Sync Now" or the status indicator pill when authentication has expired now opens the UGent SSO login window directly, rather than waiting 53 seconds in a blocked state.
  - Automatically refreshes the enrolled courses list whenever a manual sync pass is triggered.
  - Fixed out-of-sync auth status in the desktop UI and system tray: auth failure events (`auth-status-changed: false`) now immediately update the Header and show a prominent "Log in with UGent" button.
  - Added live status updates and dedicated "Sign in to UGent Ufora" action to the macOS menu bar and system tray menu whenever re-authentication is required.
- **Robust Microsoft Entra ID & UGent SSO Helper**:
  - Enhanced the auto-auth script to dispatch synthetic mouse events (`MouseEvent` + `click`) across Knockout.js and React containers on Microsoft SSO.
  - Token extraction now strictly validates JWT `exp > now + 60`, preventing stale expired tokens in `localStorage` from being mistakenly captured.
  - Intercepts all live in-page network requests for immediate Bearer token capture.
  - Fixed macOS WebKit background window throttling by transitioning from silent refresh to visible focused window within 4 seconds, bringing the window to the front and focusing it cleanly.

## [1.0.4] - 2026-10-30

### Added
- **Intelligent Automatic SSO Renewal & Account Auto-Selection**:
  - Implemented multi-stage token renewal (`ensure_valid_token`) that proactively checks token validity before every sync pass and course query.
  - Injected automated Microsoft Entra ID / UGent SSO scripting (`AUTO_AUTH_AND_TOKEN_SCRIPT`) into WebViews: automatically recognizes and clicks saved UGent account tiles and auto-confirms "Stay signed in?" prompts.
  - Automatically pops open the login window when silent renewal requires interaction, auto-selects the saved student account, extracts the refreshed token, and closes itself seamlessly without manual credential re-entry.
  - Proactive background token keep-alive worker running every 10 minutes to renew sessions expiring within 20 minutes.
  - Added a dedicated "Refresh Session" button in the Settings Account card.
- **System Startup at Boot (Launch at Login)**:
  - Added native autostart support via `tauri-plugin-autostart` across macOS (LaunchAgent), Windows (Registry), and Linux (XDG Autostart).
  - Added dynamic "Start at Login" toggle to the native macOS / desktop system tray menu.
  - Added a "System & Startup" preference card in the Settings modal with real-time OS state synchronization.
  - Added `--minimized` launch flag support so the app quietly boots into the menu bar without interrupting the student's workflow.

## [1.0.3] - 2026-10-29

### Fixed
- **Automated Multi-Platform Release Pipeline**: Resolved GitHub Actions workflow issue in reusable `tauri-release.yml` by removing reserved system secret collisions and ensuring write permissions for automated matrix bundle creation (macOS DMG, Windows installer, Linux packages).

### Changed
- **Documentation**: Generalized campus references in the showcase overview.

## [1.0.2] - 2026-10-29

### Changed
- **Catchy Student-First README**: Replaced developer-heavy documentation with an engaging, funny, human overview; extracted build-from-source guides into `BUILDING.md`.
- **Branding & License**: Updated official README banner and streamlined About metadata to pure MIT attribution without copyright headers.

## [1.0.1] - 2026-10-29

### Added
- **Native macOS Application Menu & About Panel**: Full Cocoa native About panel integration (`AboutMetadataBuilder`) with credits ("Mauro Druwel — MIT License"), authors, and version display.
- **Semantic Versioning Comparison**: Added `is_newer_version` utility ensuring only strictly newer semantic tags on GitHub prompt update notifications, preventing older repository tags from misreporting as updates.
- **Dynamic Relative Sync Formatting**: Header status displays live relative time ("Synced just now", "Synced 2 min ago", "Last synced HH:MM") with reactive 4-state indicator dots.

### Changed
- **Header Geometry**: Relocated status indicator chip to the top right directly next to the sync action buttons, leaving the left title bar clear for macOS window traffic lights.
- **Streamlined Preferences & Settings**: Moved course enrollment visibility toggles (past academic years and group spaces) into Settings, keeping the main dashboard toolbar clutter-free.

## [1.0.0] - 2026-10-28

### Added
- **Official Application Branding & Iconography**:
  - Replaced temporary icons and placeholder sparkles with the official UGent Ufora Sync squircle emblem across macOS (`icon.icns`), Windows (`icon.ico`), Linux (`icon.png`), and the desktop app header.
  - Added high-resolution assets to `assets/logo.png` and `public/app-logo.png`.
- **Course Selection Safety & Confirmation**:
  - Replaced raw "All" and "None" toggle buttons with a batch selection popover menu (`{count} active ▾`) including Invert Selection.
  - Added Material-style confirmation dialogs before executing "Disable All" or "Enable All" to eliminate accidental misclicks.
  - Added two-step confirmation on the Logout button inside Settings.
- **Course Grid View & Search Filtering**:
  - Added multi-column responsive Grid View (`LayoutGrid`) alongside classic List View (`List`), with persistent layout preference.
  - Added instant search filter for courses matching by course name, code, or ID, designed for users enrolled in many courses.
- **UGent Ufora Logout Action**:
  - Added native logout command that clears `~/.d2l/token.json` and updates session state across the app.
  - Added accessible Logout buttons in the top Header, Settings modal, and system tray menu.
- **Native Tauri v2 + Rust Architecture**: Complete ground-up rewrite replacing legacy Python daemon with an ultra-lightweight, high-performance Rust core and React 18 / TypeScript frontend. RAM consumption reduced from ~120 MB to <15 MB.
- **Native Embedded SSO Login**: Integrated native WebKit / WebView2 authentication modal with automatic session cookie extraction and persistence to `~/.d2l/token.json`. Completely eliminates external Python dependencies and manual terminal login commands.
- **Google Gemini Web UI Design System**:
  - Utility-first UI matching Google Gemini specifications with true neutral surfaces (`#131314` dark / `#ffffff` light base, `#1e1f20` / `#f0f4f9` containers).
  - Automatic OS Dark / Light theme detection via `prefers-color-scheme` with zero flash-of-unstyled-content.
  - Full pill badges, refined typography, subtle hairline borders (`rgba(255, 255, 255, 0.08)` / `rgba(0, 0, 0, 0.08)`), and restrained Material blue accents (`#a8c7fa` / `#1a73e8`).
  - Native window dragging support (`data-tauri-drag-region`).
- **Cross-Platform Cloud Storage Safety**:
  - Added Windows NTFS reparse point detection (`FILE_ATTRIBUTE_REPARSE_POINT` / `RECALL_ON_DATA_ACCESS`) alongside macOS APFS `SF_DATALESS` to prevent accidental hydration of Microsoft OneDrive Files On-Demand placeholders.
  - Guaranteed non-destructive local file safety during background sync passes.
- **Mauro Quality Gate (MQG) Verification**: 100% compliance with MQG standards (Pillars 1–6) utilizing centralized reusable workflows (`MauroDruwel/quality-gate/.github/workflows/tauri-ci.yml` and `tauri-release.yml`).

### Fixed
- **macOS Window Traffic Lights Overlap**: Fixed header icon and title overlapping native window controls (close/minimize/zoom) by applying platform-aware header inset padding (`pl-20` on macOS, `pl-6` on Windows/Linux).
- **Zero Python Artifacts**: Removed legacy Python scripts, virtualenvs, and test runners from the repository.

## [0.2.8] - 2026-10-04

### Fixed
- **Comprehensive SharePoint & OneDrive Folder Sanitization**: Resolved critical sync looping caused by folders with leading hyphens (e.g. `- Kennisclips`, `- Lesopname`, `- Algemeenheden`), degree symbols (`°Verslaggeving`, `°Verslagen`), and apostrophes (`Video's`, `Video's uitgewerkte oefeningen`). Stripped leading/trailing hyphens, periods, spaces, underscores, and normalized Unicode across all folder names.
- **Stem Trailing Dash Sanitization**: Fixed sync failures on files whose stems ended in hyphens before the extension (e.g. `Werkcollege 1-.html` -> `Werkcollege 1.html`).
- **Deep Directory & User File Migration**: Implemented recursive bottom-up folder migration in `_migrate_course_manifest()`. Renames invalid folders on disk and moves all tracked files, subdirectories, and untracked user coursework (such as lab reports, notes, photos) cleanly into the sanitized paths while updating the manifest.
- **macOS Finder `.DS_Store` Cleanup**: Enhanced empty directory cleanup to recursively remove macOS Finder `.DS_Store` metadata files so abandoned invalid folders are completely pruned rather than remaining on disk.
- **Heavy I/O Sync Loop Elimination**: Optimized incremental sync checking to skip re-reading and SHA-256 hashing large local video files when remote modification timestamps are unchanged. This stops continuous 8+ GB background disk thrashing and prevents FSEvents storms that aborted OneDrive upload gates.

## [0.2.7] - 2026-10-04

### Fixed
- **Cloud Storage & OneDrive Filename Sanitization**: Fixed an issue where filenames with trailing whitespace or dots before the file extension (e.g. `V_Meter_2 .mp4`, `Inleiding 2026 - 2027 .pptx`) were written to disk as-is, causing OneDrive and SharePoint sync engines on macOS to enter an infinite retry/sync loop.
- **OneDrive "Free Up Space" & Files On-Demand Compatibility**: Prevented automatic cloud re-hydration when users use "Free Up Space" on macOS (`stat.SF_DATALESS`) or Windows. The syncer detects evicted cloud placeholders and never opens or re-reads them to hash, keeping files in the cloud until genuine remote changes occur.
- **Auto-Migration of Unsanitized Files**: The incremental sync engine now automatically detects previously downloaded files on disk with trailing spaces or illegal characters, renames them cleanly, and updates the `.ufora_sync_manifest.json` without requiring re-downloads or manual intervention.
- **Folder & Shortcut Sanitization**: Enforced strict stripping of trailing periods, dashes, and spaces on course and module folders as well as HTML activity shortcuts to comply with Microsoft OneDrive restrictions.

## [0.2.6] - 2026-10-01

### Fixed
- **Sync Directory Migration & Portability**: Completely resolved `ValueError: '...' is not in the subpath of '...'` crashes when moving courses or changing the sync destination (e.g. from local `Documents/Ufora` to cloud storage `OneDrive-UGent/Ufora`).
- **Relative Path Manifests**: Synced file records (`.ufora_sync_manifest.json`) are now persisted using relative POSIX subpaths instead of machine-specific absolute paths, making manifests fully portable across folders, drives, and cloud storage providers.
- **Automatic Legacy Manifest Migration**: Existing manifests containing absolute paths are transparently migrated upon load, preserving all remote modification timestamps and local edit detection hashes.
- **Dynamic GUI Course Card Directory Updates**: Course folder action buttons in the Settings window immediately point to the newly selected destination directory upon saving settings without requiring an application restart.

## [0.2.5] - 2026-09-29

### Added
- **Single-Instance Application Enforcement**: Guaranteed single background daemon (`daemon.lock`) and single Settings window (`gui.lock`) per user. Launching `ufora-sync` or `ufora-sync tray` when already running focuses the existing instance instead of starting duplicate background services.
- **Inter-Process Sync Delegation**: The GUI "Sync Now" button and terminal `ufora-sync sync` command now delegate directly to the running background daemon via lightweight IPC triggers (`trigger.sync`), eliminating duplicate syncer threads and race conditions.
- **IPC Watcher Engine (`src/ufora_sync/ipc.py`)**: Seamless trigger system coordinating daemon sync requests and GUI focus across processes.

## [0.2.4] - 2026-09-29

### Added
- **Animated Syncing Icon**: Smooth rotating sync arrows animation in the macOS menu bar while sync is in progress, returning to the static clean cloud silhouette immediately when sync completes.
- **Persistent Activity Logging (`activity.log`)**: All daemon and sync activity is recorded to a persistent, bounded log file (`~/Library/Application Support/ufora-sync/activity.log`) across all processes and runs.
- **Live GUI Log Stream**: The Settings window Activity Log tab automatically tails the daemon's log in real time, so all background syncing activity is immediately visible even when running as separate processes.
- **Cross-Process Status Synchronization**: Daemon state (`service_status.txt`) is persisted to disk and polled live by the GUI window.

### Fixed
- **Stuck Tray Status Menu**: Fixed an issue where the menu item remained stuck on `● Syncing…` after sync completed by calling `icon.update_menu()` and dispatching thread-safe UI updates on macOS via `PyObjCTools.AppHelper.callAfter`.
- **Empty Settings Activity Log**: Resolved disconnect between the background tray daemon and standalone Settings GUI window by streaming the shared persistent activity log.

## [0.2.3] - 2026-09-29

### Added
- **Native macOS Menu Bar Styling (Retina Template Mode)**: Redesigned the tray icon specifically for the macOS menu bar. Uses a crisp 44x44 (2x Retina) anti-aliased template mask (`setTemplate_(True)` on `NSImage`), seamlessly blending with macOS native monochrome menu bar icons (Wi-Fi, Battery, Control Center). Automatically tints for Light and Dark modes as well as macOS desktop wallpaper accents. Status states (syncing, paused, error) feature crisp geometric cutouts. Full-color badges remain for Windows and Linux.
- **Cross-Platform Auto-Start on System Startup**: Added native background auto-launch support on user login:
  - **macOS**: Native user LaunchAgent (`~/Library/LaunchAgents/com.maurodruwel.ufora-sync.plist`).
  - **Linux**: XDG Autostart desktop entry (`~/.config/autostart/ufora-sync.desktop`).
  - **Windows**: Windows registry Run key (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- **Auto-Start GUI & CLI Controls**: Added a 1-click checkbox ("Launch Ufora Sync automatically when logging into computer") in the Settings window, and new CLI commands: `ufora-sync autostart [status|enable|disable]`.
- **Version Reporting**: Added `--version` / `-v` flag to `ufora-sync` CLI, and added a version footer in the Settings window displaying both `ufora-sync` and underlying `ufora-ai` engine versions.

## [0.2.2] - 2026-09-29

### Added
- **Silent Background Token Auto-Renewal**: Eliminates hourly session expiry prompts by automatically renewing expired or expiring tokens in the background via saved SSO session cookies.
- **`ensure_authenticated` Helper**: Proactively checks if the Brightspace token has expired or is nearing expiration (< 3 minutes) and refreshes it before any sync pass.
- **Headless UGent SSO & Entra ID Integration**: Seamlessly interacts with `ufora refresh` and Microsoft SSO account chooser to keep the sync daemon running continuously for weeks without manual user re-login.

## [0.2.1] - 2026-09-28

### Added
- **Course & Module Descriptions (`README.md`)**: Automatically converts Ufora rich text HTML descriptions on modules and folders into formatted `README.md` files in each corresponding folder.
- **Online Activity & Link Shortcuts (`.html`)**: Generates lightweight cross-platform `.html` shortcut redirect files for online activities, quizzes, external links, and assignment dropboxes so they can be opened in the browser with a double-click.
- **TOC Readme Summaries**: Online activities and links are also cataloged in the folder's `README.md` under `## 🔗 Online Activiteiten & Links` with direct clickable URLs.
- **Single TOC Fetch Optimization**: Eliminated redundant network round-trips by reusing the Brightspace TOC structure for both file extraction and description generation.
- **Configuration Toggles**: Added GUI and config settings (`sync_descriptions`, `sync_links`) to independently enable/disable README and shortcut generation.

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

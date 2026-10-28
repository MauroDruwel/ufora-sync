# Ufora Sync

![Ufora Sync Banner](assets/banner.png)

[![Mauro Quality Gate](https://img.shields.io/badge/Mauro%20Quality%20Gate-Passed-2ea44f?style=flat&logo=github)](https://github.com/MauroDruwel/quality-gate)
[![CI](https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml/badge.svg)](https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml)
[![GitHub Release](https://img.shields.io/github/v/release/MauroDruwel/ufora-sync?color=blue)](https://github.com/MauroDruwel/ufora-sync/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Author: Mauro Druwel](https://img.shields.io/badge/Author-Mauro%20Druwel-orange)](https://maurodruwel.be)

> **OneDrive-style background sync service and native desktop companion for UGent Ufora (Brightspace).**  
> Automatically downloads course files, handouts, and slides to your PC. If you edit local files, your version is preserved while still fetching the professor's updates.

> [!NOTE]
> **Disclaimer:** Ufora Sync is an independent, student-built open-source project and is **not affiliated with, maintained by, or endorsed by Ghent University (UGent) or D2L (Brightspace)**. Ufora and Brightspace are trademarks of their respective owners and are referenced solely to describe service compatibility.

Built with **Tauri v2 + Rust** for **macOS** (Apple Silicon & Intel), **Windows x64**, and **Linux x64**.

---

## ⚡ Highlights

- 🦀 **Pure Native Rust Core**: Built from scratch with Tauri v2 and Rust. Uses under 15 MB RAM and 0% idle CPU — zero Python runtime, node, or heavy dependencies required.
- 🎨 **Google Gemini Design System**: Clean, utility-first user interface adhering to Google Gemini specifications:
  - Automatic OS Dark / Light theme detection (`prefers-color-scheme`) with seamless dynamic switching.
  - True neutral backgrounds (`#131314` dark / `#ffffff` light), flat surface layering, and restrained Google Material blue accents (`#a8c7fa` / `#1a73e8`).
  - Native window dragging region with seamless macOS traffic light clearance.
- 🔐 **Native Single Sign-On (SSO)**: Built-in WebKit / WebView2 login modal with automatic session cookie extraction and persistence to `~/.d2l/token.json`. No external CLIs or browser extensions required.
- ☁️ **OneDrive & Cloud Storage Aware**:
  - Detects macOS APFS `SF_DATALESS` and Windows NTFS reparse point placeholders (`RECALL_ON_DATA_ACCESS`).
  - Never forces cloud re-hydration during SHA-256 integrity checks when using OneDrive "Free Up Space".
- 🛡️ **Guaranteed Non-Destructive File Safety**:
  - `duplicate` *(default)*: Stashes your local modifications as `<filename>_edited.<ext>` and fetches the professor's updated handout to the original path.
  - `skip`: Preserves your local modifications and leaves them untouched.
  - `overwrite`: Replaces local files when remote revisions are released.
  - User-created files, personal notes, and untracked coursework are **never deleted**.
- 📁 **Preserved Module Structure**: Keeps exact course hierarchies (e.g. `Statica / Lessen / H1_Statica.pdf`) instead of flat lists.
- 📝 **Module & Folder Descriptions**: Converts professor announcements and folder descriptions into local `README.md` files.
- 🔗 **Online Activities & Quicklinks**: Generates lightweight `.html` shortcut redirect files for quizzes, Wooclap, dropboxes, and external links.
- 🚀 **Fast Incremental Sync**: Checks remote Brightspace `LastModifiedDate` timestamps to skip unchanged files instantly without redundant network transfers.
- 🖥️ **System Tray & Background Sync**: Runs silently in your menu bar or system tray with live status indicators and real-time activity logs.
- 🏛️ **Mauro Quality Gate (MQG) Verified**: 100% compliance with MQG standards (Pillars 1–6) utilizing centralized reusable CI/CD pipelines.

---

## 🚀 Installation & Downloads

Pre-built binaries for macOS, Windows, and Linux are available directly on the **[Releases](https://github.com/MauroDruwel/ufora-sync/releases)** page:

- **macOS**: `.dmg` (Universal binary for Apple Silicon M1-M4 and Intel x86_64)
- **Windows**: `.msi` / `.exe` installer (x64)
- **Linux**: `.AppImage` / `.deb` (x64)

---

## 💻 Building from Source

### Prerequisites
- [Node.js](https://nodejs.org/) (v20+ or v22 LTS recommended)
- [pnpm](https://pnpm.io/) (v10+)
- [Rust & Cargo](https://rustup.rs/) (v1.75+)
- Platform-specific build tools:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Windows**: Visual Studio 2022 C++ Build Tools & WebView2
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Development
```bash
git clone https://github.com/MauroDruwel/ufora-sync.git
cd ufora-sync

# Install frontend dependencies
pnpm install

# Run desktop app in development mode
pnpm tauri dev
```

### Production Build
```bash
# Lint & typecheck
pnpm run lint
pnpm run typecheck

# Run test suites
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml

# Build native desktop bundle
pnpm tauri build
```

---

## ⚙️ Configuration

Configuration is stored in standard OS user application data directories:
- **macOS**: `~/Library/Application Support/ufora-sync/config.json`
- **Linux**: `~/.config/ufora-sync/config.json`
- **Windows**: `%APPDATA%\ufora-sync\config.json`

Example `config.json`:
```json
{
  "sync_dir": "/Users/student/Library/CloudStorage/OneDrive-UGent/Ufora",
  "interval_minutes": 30,
  "enabled_courses": [
    "1377778",
    "1386250"
  ],
  "conflict_strategy": "duplicate",
  "duplicate_suffix": "_edited",
  "sync_descriptions": true,
  "sync_links": true,
  "auto_start_tray": true
}
```

---

## 🏛️ Mauro Quality Gate (MQG)

This project strictly adheres to the **[Mauro Quality Gate](https://github.com/MauroDruwel/quality-gate)** standards:

- **Strict Type Safety**: Strict TypeScript (`noImplicitAny: true`) and zero-warning Rust (`cargo clippy --all-targets -- -D warnings`).
- **Automated Verification**: Comprehensive unit tests covering incremental synchronization, conflict resolution, and token persistence.
- **Centralized Reusable Workflows**: CI/CD runs on every push and pull request via `MauroDruwel/quality-gate/.github/workflows/tauri-ci.yml`.

---

## ⚖️ Disclaimer

**Ufora Sync is not affiliated with, maintained by, or endorsed by Ghent University or D2L.** This is an unofficial, student-built open-source project. Ufora and Brightspace are referenced solely to identify the educational platform this project interoperates with.

---

## 📜 License

MIT License © [Mauro Druwel](https://maurodruwel.be). See [LICENSE](LICENSE) for details.

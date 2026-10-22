# Ufora Sync

![Ufora Sync Banner](assets/banner.png)

[![Mauro Quality Gate](https://img.shields.io/badge/Mauro%20Quality%20Gate-Passed-2ea44f?style=flat&logo=github)](https://github.com/MauroDruwel/quality-gate)
[![CI](https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml/badge.svg)](https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml)
[![GitHub Release](https://img.shields.io/github/v/release/MauroDruwel/ufora-sync?color=blue)](https://github.com/MauroDruwel/ufora-sync/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Author: Mauro Druwel](https://img.shields.io/badge/Author-Mauro%20Druwel-orange)](https://maurodruwel.be)

> **Native background sync service and desktop companion for UGent Ufora (Brightspace).**  
> Built with **Tauri v2**, **Rust**, and **React**. Automatically synchronizes course documents, announcements, and links in the background with local-first file preservation.

> [!NOTE]
> **Disclaimer:** Ufora Sync is an independent, student-built open-source project and is **not affiliated with, maintained by, or endorsed by Ghent University (UGent) or D2L (Brightspace)**. Ufora and Brightspace are trademarks of their respective owners and are referenced solely to describe service compatibility.

Built for **macOS**, **Windows**, and **Linux**.

---

## ⚡ Highlights

- ☁️ **Native Rust Sync Engine**: High-performance, memory-efficient background sync daemon powered by Tokio and asynchronous Rust.
- 🔄 **Silent Token Management**: Seamlessly interoperates with Brightspace SSO sessions from `~/.d2l`.
- 🖥️ **System Tray & Menu Bar Companion**: Live sync indicators (Idle, Syncing, Up to date, Needs Login) with one-click actions.
- 🎨 **Modern Desktop GUI**: Sleek React + Tailwind UI with macOS vibrancy styling to manage courses, sync directories, and intervals.
- 📁 **Preserved Module Structure**: Keeps exact course hierarchies intact (`Course / Lectures / Chapter1.pdf`) instead of flat dumps.
- 📝 **Module & Folder Descriptions**: Converts professor announcements and folder descriptions into local `README.md` files.
- 🔗 **Online Activities & Quicklinks**: Generates lightweight `.html` shortcut redirects (quizzes, Wooclap, external links) that launch directly in your default browser.
- 🚀 **Incremental Sync**: Checks remote Brightspace timestamps to skip unchanged files instantly without downloading them.
- 🛡️ **Zero Stray Temp Files**: Downloads are staged safely in temporary storage and committed atomically.
- 💾 **Non-Destructive Local Edits**:
  - `duplicate` *(default)*: Preserves edited files as `<filename>_edited.<ext>` and fetches professor updates cleanly.
  - `skip`: Keeps local edits and skips downloading updates for those specific files.
  - `overwrite`: Replaces local files unconditionally.
- 🔒 **Process-Level Single Instance**: Handled through native Tauri single-instance plugins to prevent collisions.
- 🏛️ **Mauro Quality Gate (MQG) Verified**: 100% compliance with MQG standards (Pillars 1–6).

---

## 🚀 Getting Started

### Prerequisites

- **Node.js**: `22.x` or later
- **pnpm**: `10.x` or later
- **Rust toolchain**: `stable` (`cargo`, `rustc`)

### Development

```bash
git clone https://github.com/MauroDruwel/ufora-sync.git
cd ufora-sync

# Install frontend dependencies
pnpm install

# Run desktop application in development mode
pnpm tauri dev
```

### Production Build

```bash
# Build desktop release bundles (.dmg, .app, .deb, .msi, etc.)
pnpm tauri build
```

---

## ⚙️ Configuration

Configuration is stored in standard OS user application data directories:
- **macOS**: `~/Library/Application Support/be.maurodruwel.ufora-sync/config.json`
- **Linux**: `~/.config/be.maurodruwel.ufora-sync/config.json`
- **Windows**: `%APPDATA%\be.maurodruwel.ufora-sync\config.json`

Example `config.json`:
```json
{
  "sync_dir": "/Users/student/Documents/Ufora",
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

## 🧪 Testing & Verification

All automated tests and linting suites pass with zero warnings:

```bash
# Frontend unit tests
pnpm test

# Frontend linting
pnpm lint

# Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# Rust strict linting
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

---

## ⚖️ Disclaimer

**Ufora Sync is not affiliated with, maintained by, or endorsed by Ghent University or D2L.** This is an unofficial, student-built open-source project. Ufora and Brightspace are referenced solely to identify the educational platform this project interoperates with.

---

## 📜 License

MIT License © [Mauro Druwel](https://maurodruwel.be). See [LICENSE](LICENSE) for details.

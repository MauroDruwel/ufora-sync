# Ufora Sync

[![Mauro Quality Gate](https://img.shields.io/badge/Mauro%20Quality%20Gate-Passed-2ea44f?style=flat&logo=github)](https://github.com/MauroDruwel/quality-gate)
[![CI](https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml/badge.svg)](https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Author: Mauro Druwel](https://img.shields.io/badge/Author-Mauro%20Druwel-orange)](https://maurodruwel.be)

> **OneDrive-style background sync service and system tray application for UGent Ufora (Brightspace).**  
> Automatically downloads course files, handouts, and slides to your PC. If you edit local files, your version is preserved while still fetching the professor's updates.

Built for **macOS**, **Linux**, and **Windows**.

---

## ⚡ Highlights

- ☁️ **OneDrive-like Sync Daemon**: Runs silently in the background and continuously synchronizes your enrolled courses.
- 🖥️ **System Tray / Menu Bar Icon**: Live status indicators (Idle, Syncing, Up-to-date, Needs Login) with quick right-click actions.
- 🛡️ **Non-Destructive Local Edits**:
  - `duplicate` *(default)*: Stashes your edited copy as `<filename>_edited.<ext>` and downloads the professor's fresh version to the original path.
  - `skip`: Keeps your local modifications and skips downloading updates for those specific files.
  - `overwrite`: Replaces local files unconditionally.
- 🎓 **Course Selection Hub**: Clean dark GUI to toggle which courses to sync and open local course folders with one click.
- 🔒 **Local & Private**: No third-party servers. Your session stays on your machine (`~/.d2l`).
- 🏛️ **Mauro Quality Gate (MQG) Verified**: Full compliance with MQG standards (Pillars 1–6).

---

## 🚀 Quick Start

### 1. Requirements
- Python 3.11+
- Chromium browser (Google Chrome, Chromium, Brave, or Edge for SSO login)

### 2. Installation
```bash
git clone https://github.com/MauroDruwel/ufora-sync.git
cd ufora-sync

# Install with pip
pip3 install -e .
```

### 3. Log into UGent
```bash
ufora login
```
A browser window will open — sign into UGent SSO as you normally do. Your token will be securely saved under `~/.d2l/token.json`.

---

## 💻 Usage

### Default (Menu Bar / Tray + Auto Sync)
```bash
ufora-sync
```
Launches the system tray app and starts the background sync service. If it's your first run, the settings window will open automatically so you can pick which courses to sync.

### Open Settings & Course Selection Window
```bash
ufora-sync gui
```

### Run as Headless Daemon (e.g. systemd or background script)
```bash
ufora-sync service
```

### Instant One-Shot Sync
```bash
ufora-sync sync
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
  "sync_dir": "/Users/student/Documents/Ufora",
  "interval_minutes": 30,
  "enabled_courses": [
    "1377778",
    "1386250"
  ],
  "conflict_strategy": "duplicate",
  "duplicate_suffix": "_edited",
  "auto_start_tray": true
}
```

---

## 🧪 Testing

```bash
python3 -m pytest tests/ -v
python3 -m ruff check
```

---

## 📜 License

MIT License © [Mauro Druwel](https://maurodruwel.be). See [LICENSE](LICENSE) for details.

# Ufora Sync

> One-way sync GUI for UGent Ufora course files. Download slides, PDFs, and materials to your PC — your local edits are **never overwritten**.

Built on top of [ufora-ai](https://github.com/LiamVDB1/ufora-ai).

## Features

- 🎓 Lists all your current Ufora courses
- 📁 Shows the full content tree per course
- ☑️ Checkboxes to pick exactly what to sync
- 📥 One-way sync: Ufora → your PC only
- 🛡️ **Never overwrites local edits** (tracked via SHA-256 manifest)
- 🗂️ You pick the destination folder

## Requirements

- Python 3.11+
- [uv](https://github.com/astral-sh/uv)
- Chromium browser (for `ufora login`)

## Install

```bash
git clone https://github.com/MauroDruwel/ufora-sync
cd ufora-sync
uv sync
```

## Usage

```bash
# First login to Ufora (opens a browser)
uv run python -m ufora_cli.d2l_entry login

# Launch the GUI
uv run ufora-sync
```

## How "no overwrite" works

When a file is first downloaded, its SHA-256 hash is stored in `.ufora_sync_manifest.json` next to the synced files. On the next sync:

- If the file's hash **matches** the stored hash → safe to update from Ufora
- If the file's hash **differs** from stored → you edited it → **skipped**, your version is kept

## License

MIT — Mauro Druwel

<p align="center">
  <img src="assets/banner.png" alt="Ufora Sync Banner" width="100%" />
</p>

<p align="center">
  <a href="https://github.com/MauroDruwel/ufora-sync/releases"><img src="https://img.shields.io/github/v/release/MauroDruwel/ufora-sync?color=2563eb&label=version" alt="Latest Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <a href="https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml"><img src="https://github.com/MauroDruwel/ufora-sync/actions/workflows/ci.yml/badge.svg" alt="CI status" /></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-neutral" alt="Cross-Platform" />
</p>

---

### Because clicking through 14 subfolders on Ufora every Sunday evening is nobody's hobby.

You know the drill: it’s 8:15 AM, you’re sitting in an auditorium on campus, eduroam is having an existential crisis, and the prof announces: *"I uploaded the updated exercises 10 minutes ago on Ufora."*

Now you're furiously refreshing Brightspace on 4G, clicking through *Inhoud → Lesmateriaal → Werkcolleges → Week 4 → Definitief → echt_deze_v2.pdf*, waiting for a 2 MB file to download.

**Ufora Sync fixes that.** 

It runs silently in the background and automatically syncs all your course materials, slides, code, and handouts directly into a local folder on your computer (like your UGent OneDrive or local Documents).

When the professor updates a slide deck, it’s already on your laptop before you even sit down.

<p align="center">
  <img src="assets/app-screenshot.png" alt="Ufora Sync Desktop Interface" width="88%" />
</p>

---

## 🚀 Download & Run

No command line required. Just download, open, and log in with your UGent account:

| Platform | Download Link | Notes |
| :--- | :--- | :--- |
| 🍏 **macOS** | **[Download .dmg](https://github.com/MauroDruwel/ufora-sync/releases/latest)** | Apple Silicon & Intel |
| 🪟 **Windows** | **[Download .msi](https://github.com/MauroDruwel/ufora-sync/releases/latest)** | Windows 10 / 11 (x64) |
| 🐧 **Linux** | **[Download .AppImage](https://github.com/MauroDruwel/ufora-sync/releases/latest)** | Ubuntu, Fedora, Arch, Debian |

*Prefer building it yourself? See [BUILDING.md](BUILDING.md).*

Ufora Sync checks for updates in the background on launch. You can also check
under **Settings → Software Updates**.

---

## ✨ What makes it actually good?

- ⚡ **Lightweight & Native (Rust + Tauri v2)**  
  Built with Rust and the operating system webview instead of bundling a full browser engine. Background polling stays idle between sync checks.

- ✍️ **Protects your notes by default**<br>
  Took handwritten Apple Pencil notes or typed annotations onto a slide deck? The default conflict strategy detects local changes and preserves them (`Les1_edited.pdf`) while grabbing the latest version from the prof. An explicit overwrite mode remains available in Settings.

- 🧹 **No 60-course clutter**  
  UGent Brightspace loves showing courses from 3 years ago, dead group pages, and random faculty portals. Ufora Sync automatically focuses on your **current academic-year courses**, with easy toggles in Settings if you ever need an old archive.

- ☁️ **OneDrive-friendly (Files On-Demand aware)**  
  If you sync into `OneDrive - UGent` and use *"Free Up Space"*, Ufora Sync won't force OneDrive to redownload your whole disk just to check if files changed.

- 📝 **Clean folder structure**  
  Preserves the exact hierarchy your professors made (e.g. `Mechanica / Hoofdstuk 2 / Oefeningen.pdf`). No cryptic `D2L_Export_849204.zip` archives or broken file names with `%20`.

- 🔗 **Online links & quizzes as shortcuts**  
  Professor put a Wooclap link, textbook URL, or online quiz inside the content tree? It gets saved as a handy `.html` shortcut file that opens straight to the right page in your browser.

- 🎨 **Clean Google Gemini UI**  
  Looks and feels like a native macOS/Windows utility with automatic Dark/Light mode, live status dots, and zero annoying popups.

---

## 📸 In Action

| ⚙️ Preferences & Conflict Settings | ⚡ Live Incremental Sync Logs |
| :---: | :---: |
| <img src="assets/settings-screenshot.png" width="100%" alt="Preferences & Settings" /> | <img src="assets/activity-log-screenshot.png" width="100%" alt="Activity Log" /> |

---

## 🛠 For Developers & Compiling

Want to hack on the codebase, add a feature, or build from source?  
Check out the simple guide in **[BUILDING.md](BUILDING.md)**.

---

## ⚖️ Disclaimer & License

Ufora Sync is an independent, student-built open-source project and is **not affiliated with, maintained by, or endorsed by Ghent University (UGent) or D2L (Brightspace)**. Ufora and Brightspace are trademarks of their respective owners.

Released under the **[MIT License](LICENSE)** · Built with ❤️ by **[Mauro Druwel](https://maurodruwel.be)**.

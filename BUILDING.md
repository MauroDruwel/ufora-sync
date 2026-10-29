# Building Ufora Sync from Source

If you want to contribute, tinker with the code, or build custom binaries yourself, here is how to get the project running locally.

---

## 🛠 Prerequisites

- **[Node.js](https://nodejs.org/)** (v20+ or v22 LTS)
- **[pnpm](https://pnpm.io/)** (v10+)
- **[Rust & Cargo](https://rustup.rs/)** (v1.75+)
- Platform-specific tools:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Windows**: Visual Studio 2022 C++ Build Tools & WebView2 runtime
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

---

## 🚀 Quick Setup

```bash
# 1. Clone the repository
git clone https://github.com/MauroDruwel/ufora-sync.git
cd ufora-sync

# 2. Install frontend dependencies
pnpm install

# 3. Start development mode with hot reload
pnpm tauri dev
```

---

## 🧪 Running Tests & Quality Gate

```bash
# Frontend linting, typecheck & vitest
pnpm run lint
pnpm run typecheck
pnpm test

# Rust backend tests & clippy
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
```

---

## 📦 Production Builds

```bash
# Produces native installer (.dmg, .msi, or .AppImage) in src-tauri/target/release/bundle/
pnpm tauri build
```

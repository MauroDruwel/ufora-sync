# Building Ufora Sync from Source

If you want to contribute, tinker with the code, or build custom binaries yourself, here is how to get the project running locally.

---

## 🛠 Prerequisites

- **[Node.js](https://nodejs.org/)** (v20+ or v22 LTS)
- **[pnpm](https://pnpm.io/)** (v10+)
- **[Rust & Cargo](https://rustup.rs/)** (v1.80+)
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
# Version consistency, frontend linting, typecheck, tests, and production build
pnpm run check:versions
pnpm run lint
pnpm run typecheck
pnpm test
pnpm run build

# Browser E2E (Chromium; no UGent credentials needed)
pnpm exec playwright install chromium
pnpm run test:e2e

# Rust backend formatting, checks, clippy, and tests
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo check --manifest-path src-tauri/Cargo.toml --all-targets --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked
```

---

The Playwright suite runs the real frontend in Chromium with mocked Tauri
commands and events. It does not launch the native desktop shell or contact Ufora.

## 📦 Production Builds

```bash
# Produces a native installer for your host architecture.
pnpm tauri build
```

Builds without an explicit target are written below
`src-tauri/target/release/bundle/`. Builds with `--target` are written below
`src-tauri/target/<target>/release/bundle/`.

The release workflow produces four installers:

- macOS Apple Silicon: `aarch64-apple-darwin` (`.dmg`)
- macOS Intel: `x86_64-apple-darwin` (`.dmg`)
- Windows x64: `x86_64-pc-windows-msvc` (`.msi`)
- Linux x64: `x86_64-unknown-linux-gnu` (`.AppImage`)

To reproduce one of those builds locally, install the Rust target and pass it to
Tauri. For example, on an Apple Silicon Mac:

```bash
rustup target add aarch64-apple-darwin
pnpm tauri build --target aarch64-apple-darwin --bundles dmg
```

Locally built installers are unsigned unless you configure the platform's code
signing credentials.

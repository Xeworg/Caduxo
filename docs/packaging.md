# Packaging Guide — Caduxo

This document covers runtime requirements, build prerequisites, and distribution options for Caduxo on Linux and Windows. It is the authoritative packaging reference for contributors and downstream packagers.

> **macOS**: Not a primary target for v1. The `tauri.conf.json` bundle target is `"all"`, which includes `.app` on macOS. The same WebKit/WebView assumptions below apply: macOS provides WebKit natively and requires no additional runtime.

---

## Runtime Dependencies

### Windows — WebView2

Caduxo uses Tauri v2, which embeds the operating system's native web-view for the UI layer.

| Component | Version | Notes |
| --------- | ------- | ----- |
| WebView2 Runtime | Evergreen (auto-updated) | Pre-installed on Windows 10 1803+ and all Windows 11 |
| Fallback | WebView2 Evergreen Bootstrapper | Installer-time fallback when configured and network access is available |

**No administrator rights are required** to run Caduxo when WebView2 is already present. On current Windows 10/11 installations this is normally true. If WebView2 is absent, the bundled/bootstrapper path may require network access and can be blocked by enterprise policy.

**Verification**: Check for the Evergreen Runtime in Windows Settings, confirm `msedgewebview2.exe` exists under `%ProgramFiles(x86)%\Microsoft\EdgeWebView\Application\`, or verify the WebView2 runtime registry entry. If absent, install the Evergreen Runtime before launching Caduxo, or use the installer path produced by Tauri on Windows.

### Linux — WebKitGTK

Tauri v2 on Linux uses WebKitGTK for the UI layer. This is a shared library dependency that must be present on the target system.

| Package (Debian/Ubuntu) | Package (Fedora/RHEL) | Package (Arch) |
| ------------------------ | ---------------------- | -------------- |
| `libwebkit2gtk-4.1-dev` | `webkit2gtk4.1-devel` | `webkit2gtk4.1` |
| `libgtk-3-dev` | `gtk3-devel` | `gtk3` |
| `libssl-dev` | `openssl-devel` | `openssl` |
| `libsoup-3.0-dev` | `libsoup3-devel` | `libsoup3` |
| `gdk-pixbuf-2.0-dev` | `gdk-pixbuf2-devel` | `gdk-pixbuf2` |
| `libcairo2-dev` | `cairo-devel` | `cairo` |
| `libpango1.0-dev` | `pango-devel` | `pango` |
| `libatk1.0-dev` | `atk-devel` | `atk` |
| `libglib2.0-dev` | `glib2-devel` | `glib2` |

**Minimum WebKitGTK version**: 4.1 (for Tauri v2 compatibility).

**No administrator rights are required** to run Caduxo on Linux with a system-provided WebKitGTK. For portable/sandboxed use, see the AppImage section below.

---

## Building from Source

### Prerequisites

| Tool | Version | Purpose |
| ---- | ------- | ------- |
| Rust | ≥ 1.77.2 | Backend compilation |
| Node.js | ≥ 18 | Frontend build |
| npm | any recent | Package management |
| Tauri CLI | v2 | `npm install -D @tauri-apps/cli` |

#### Linux build dependencies

```bash
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev \
  libsoup-3.0-dev gdk-pixbuf-2.0-dev libcairo2-dev \
  libpango1.0-dev libatk1.0-dev libglib2.0-dev patchelf

# Fedora
sudo dnf install webkit2gtk4.1-devel gtk3-devel openssl-devel \
  libsoup3 gdk-pixbuf2-devel cairo-devel pango-devel \
  atk-devel glib2-devel patchelf
```

`patchelf` is required for AppImage bundling but not for `.deb`/`.rpm`.

#### Windows build dependencies

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the "Desktop development with C++" workload. The Windows SDK and MSVC toolchain are required for the `x86_64-pc-windows-msvc` Rust target.

### Build Commands

```bash
# 1. Install frontend dependencies
npm install

# 2. Development build (fast, unoptimised)
npm run tauri dev

# 3. Production build (optimised binary + installers)
npm run tauri build
```

**Output artifacts** (Linux, from `src-tauri/target/release/bundle/`):

| Artifact | Path | Notes |
| -------- | ----- | ----- |
| Raw binary | `target/release/caduxo` | Standalone executable, requires WebKitGTK |
| Debian package | `bundle/deb/Caduxo_0.1.0_amd64.deb` | Installs binary + creates desktop entry |
| RPM package | `bundle/rpm/Caduxo-0.1.0-1.x86_64.rpm` | Installs binary + creates desktop entry |
| AppImage | `bundle/appimage/Caduxo_0.1.0_amd64.AppImage` | Portable; includes WebKitGTK runtime. **Requires `linuxdeploy`** at build time (see below) |

**Output artifacts** (Windows, from `src-tauri/target/release/bundle/`):

| Artifact | Path | Notes |
| -------- | ----- | ----- |
| Raw binary | `target/release/Caduxo.exe` | Standalone; requires WebView2 to be present or installable |
| MSI installer | `bundle/msi/Caduxo_0.1.0_x64.msi` | Standard Windows installer |
| NSIS installer | `bundle/nsis/Caduxo_0.1.0_x64-setup.exe` | Click-through installer |

---

## Distribution Options

### User-Level Install (No Admin Rights)

#### Linux — .deb / .rpm

Both packages are standard system packages. Installing them through the system package manager usually requires administrator privileges:

```bash
# Debian / Ubuntu
sudo dpkg -i Caduxo_0.1.0_amd64.deb

# Fedora / RHEL
sudo rpm -i Caduxo-0.1.0-1.x86_64.rpm
```

This does not affect normal execution: after installation, running Caduxo is a user-level operation. For no-admin distribution, prefer the raw binary or AppImage options below.

#### Windows — MSI / NSIS

The raw `.exe` is the safest no-admin distribution path. Installer behaviour depends on the bundle format and Windows policy:

```powershell
# MSI with an explicit user-writable location
msiexec /i Caduxo_0.1.0_x64.msi INSTALLDIR="%LOCALAPPDATA%\Caduxo"

# NSIS silent install to a user-writable location
.\Caduxo_0.1.0_x64-setup.exe /S /D="%LOCALAPPDATA%\Caduxo"
```

If WebView2 is absent, install or bootstrap the Evergreen Runtime before launch; this may require network access and can be restricted by enterprise policy.

---

### Portable / No-Install Run

#### Linux — AppImage

AppImage bundles Caduxo with its WebKitGTK runtime in a single portable file. No installation, no root, no package manager.

**Build requirement**: `linuxdeploy` must be present at build time:

```bash
# Download linuxdeploy (one-time)
wget -q "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
chmod +x linuxdeploy-x86_64.AppImage
export PATH="$PWD:$PATH"

# Build
npm run tauri build
# AppImage will be produced at bundle/appimage/
```

**Runtime**: Download the `.AppImage` file, make it executable, and run it:

```bash
chmod +x Caduxo_0.1.0_amd64.AppImage
./Caduxo_0.1.0_amd64.AppImage
```

The AppImage runtime mounts or extracts its payload to a runtime-managed temporary/cache location. The exact path is implementation-dependent (for example `/tmp/.mount_*` for native AppImage execution or an `appimage-run` cache path when using that wrapper). Do not rely on it for persistent app data.

> **AppImage bundler status (as of this writing)**: The AppImage bundler depends on external `linuxdeploy` tooling. Optional GTK/media plugins may be needed only when the bundle configuration enables additional framework bundling. If AppImage bundling fails, fall back to the raw binary + WebKitGTK system dependency.

#### Linux — Raw Binary

The raw binary at `target/release/caduxo` is fully functional on any Linux system with WebKitGTK 4.1 installed:

```bash
./target/release/caduxo
```

No installation needed. Data is stored at `~/.local/share/caduxo/` by default.

#### Windows — Portable .exe

The raw `Caduxo.exe` from the `target/release/` directory is portable:

1. Copy `Caduxo.exe` to any folder.
2. Run it. If WebView2 is absent, install the Evergreen Runtime or use the installer path produced by Tauri on Windows.
3. Data is stored at `%APPDATA%\com.caduxo.app\` by default.

No installer required. No registry modifications.

---

## Cross-Compilation

### Building Windows Binaries

Preferred strategy: build Windows artifacts on a Windows runner with Visual Studio Build Tools installed.

```powershell
npm install
npm run tauri build
```

This is the only supported path for validating the full Windows bundle set (`.exe`, `.msi`, and NSIS installer) because those installers depend on Windows-specific tooling.

Linux cross-compilation is possible only for lower-level Rust checks and requires a dedicated Windows GNU target/toolchain. Treat it as a smoke check, not as release validation:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cd src-tauri
cargo check --target x86_64-pc-windows-gnu
```

Do not mark Windows packaging validated until `npm run tauri build` has passed on a Windows machine or CI runner.

---

## Build Validation Results (Linux, Fedora 44)

The following commands were run and passed on a standard Linux desktop environment:

| Command | Result | Output |
| -------- | ------ | ------ |
| `npx tsc --noEmit` | ✅ Pass | No TypeScript errors |
| `npm run build` | ✅ Pass | Frontend built in ~926ms |
| `cargo check` | ✅ Pass | 19 pre-existing warnings (dead code); 0 errors |
| `cargo check --release --lib` | ✅ Pass | Binary compiled in ~58s |
| `cargo clippy --lib --tests` | ✅ Pass | 0 clippy errors |
| `cargo test --lib` | ✅ Pass | 236 tests passed |
| `npm run tauri build` | ✅ Pass | Binary + `.deb` + `.rpm` + `.AppImage` built successfully after refreshing Tauri's cached `linuxdeploy` binary. |

**Binary size**: 21 MB (stripped release binary).

**AppImage size**: 107 MB.

---

## Data Storage Locations

| OS | Default path | Notes |
| -- | ------------ | ----- |
| Linux | `~/.local/share/caduxo/` | Follows XDG Base Directory spec |
| Windows | `%APPDATA%\com.caduxo.app\` | Per-user roaming app data, no admin needed |
| AppImage | Runtime-managed mount/cache path | Extracted payload location varies by AppImage runtime; app data still goes to `~/.local/share/caduxo/` |

The database file is `caduxo.db` (with optional `.db-wal` and `.db-shm` sidecars in WAL mode). Logs are written to `logs/` under the same base directory.

---

## Dependency Summary

| Dependency | Source | Required by |
| ---------- | ------ | ----------- |
| WebView2 Runtime | Windows pre-installed / bootstrapper | Windows UI |
| WebKitGTK 4.1 | System package (Linux) | Linux UI |
| GTK 3 | System package (Linux) | Linux UI |
| SQLite | Bundled (via `rusqlite` / `sqlx`) | All platforms |
| OpenSSL | System package | HTTPS-capable Tauri plugins |

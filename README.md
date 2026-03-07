<div align="center">

# WRATH

**One command. Process obliterated.**

[![CI](https://github.com/AymericChaverot/wrath/actions/workflows/ci.yml/badge.svg)](https://github.com/AymericChaverot/wrath/actions/workflows/ci.yml)
[![Release](https://github.com/AymericChaverot/wrath/actions/workflows/release.yml/badge.svg)](https://github.com/AymericChaverot/wrath/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)](https://www.rust-lang.org/)

---

A surgical, zero-compromise CLI tool to kill any process by **name** or **port**.

Built in Rust for speed, safety, and reliability.

</div>

---

## The Problem

You've been there. Port `3000` is taken. Some ghost process won't let go. You open Task Manager, hunt through the list, maybe run `netstat`, copy a PID, then `taskkill`... five steps for what should be one.

**Wrath ends that.**

## Installation

### Quick install (recommended)

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/AymericChaverot/wrath/main/scripts/install.sh | sh
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/AymericChaverot/wrath/main/scripts/install.ps1 | iex
```

The scripts always download the **latest release** from GitHub.

### From source

```bash
git clone https://github.com/AymericChaverot/wrath.git
cd wrath
cargo install --path .
```

Requires [Rust](https://www.rust-lang.org/tools/install) (stable toolchain).

### Installation paths

| Platform | Method | Install path |
|----------|--------|-------------|
| macOS / Linux | Install script | `/usr/local/bin/wrath` |
| Windows | Install script | `%USERPROFILE%\.wrath\bin\wrath.exe` |
| Any | `cargo install` | `~/.cargo/bin/wrath` |

The install scripts automatically add the binary to your `PATH`. On Windows, restart your terminal after first install.

### Updating

Run the install script again — it always fetches the latest release and overwrites the existing binary.

Wrath also checks for updates automatically after each run. If a newer version is available, you'll see a notification:

```
[i] Update available: 0.1.0 -> 0.2.0
    Run the install script again to update, or visit:
    https://github.com/AymericChaverot/wrath/releases/latest
```

## Usage

```
wrath <target>
```

The target is either a **port number** or a **process name**. Wrath figures out which one you mean.

### Kill a process by port

```bash
wrath 3000
```

```
[🔥] Wrath unleashed: Process 'node' (3 processes) using port 3000 has been obliterated.
```

Finds the process listening on port 3000, kills it and all of its child processes.

### Kill a process by name

```bash
wrath node
```

```
[🔥] Wrath unleashed: Process 'node' (2 processes) has been obliterated.
```

Finds every process matching the name, kills each one along with their child processes.

### Version

```bash
wrath --version
```

## How It Works

```
wrath <target>
  │
  ├─ Is target a number (1-65535)?
  │   ├─ Yes → Resolve port to PID → Kill process + children
  │   └─ No  → Search processes by name → Kill matches + children
  │
  └─ Output result → Check for updates
```

1. **Argument parsing** — `clap` handles CLI input and validation.
2. **Target detection** — Automatically distinguishes ports from process names.
3. **Port resolution** — Uses `netstat` (Windows) or `lsof` (Unix) to map ports to PIDs.
4. **Process discovery** — `sysinfo` provides cross-platform process enumeration.
5. **Recursive kill** — Kills the target and all descendant processes.
6. **Update check** — Queries GitHub Releases API for the latest version.

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux    | x86_64      | Fully supported |
| macOS    | x86_64      | Fully supported |
| macOS    | aarch64 (Apple Silicon) | Fully supported |
| Windows  | x86_64      | Fully supported |

## CI/CD

Every push and pull request triggers the CI pipeline:

- **Format** — `cargo fmt --check`
- **Lint** — `cargo clippy -D warnings`
- **Test** — Cross-platform tests on Linux, macOS, and Windows
- **Coverage** — `cargo-llvm-cov` with 80%+ line coverage
- **Build** — Release builds for all supported platforms

Pushing a version tag (`v*`) triggers the release pipeline:

- Builds optimized binaries for all 4 platform targets
- Packages them as `.tar.gz` (Unix) or `.zip` (Windows)
- Creates a GitHub Release with auto-generated release notes
- Attaches all binaries to the release

## Development

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo clippy -- -D warnings
```

### Format

```bash
cargo fmt
```

### Coverage

```bash
cargo llvm-cov
```

Current coverage: **80%+** line coverage.

### Creating a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

The CD pipeline handles the rest.

## Architecture

```
src/
├── main.rs      Entry point — wires CLI to business logic
├── cli.rs       Argument parsing and target detection
├── error.rs     Domain error types
├── port.rs      Port-to-PID resolution (platform-aware)
├── process.rs   Process discovery, child collection, and termination
└── update.rs    Auto-update version check via GitHub API

scripts/
├── install.sh   Installer for macOS and Linux
└── install.ps1  Installer for Windows (PowerShell)
```

Each module has a single responsibility. No unsafe code.

## Dependencies

| Crate | Purpose |
|-------|---------|
| [`clap`](https://crates.io/crates/clap) | Command-line argument parsing |
| [`sysinfo`](https://crates.io/crates/sysinfo) | Cross-platform process management |
| [`ureq`](https://crates.io/crates/ureq) | HTTP client for update checks |
| [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) | JSON deserialization for GitHub API |

## License

[MIT](LICENSE) — Aymeric CHAVEROT

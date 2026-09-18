# Developing Recall Gate

Recall Gate is a Rust workspace. The supported workflow for every contributor is a pinned toolchain, ordinary `cargo` commands, and CI on Ubuntu that installs the same system libraries documented below.

[Nix](#optional-nix-development-shell) is optional. It mirrors these dependencies for reproducible GTK and Wayland versions.

## Prerequisites

| Requirement | Notes |
| --- | --- |
| Rust | Installed via [rustup](https://rustup.rs/). The repo pins the version in `rust-toolchain.toml`. |
| Linux | Full stack (daemon, Wayland/X11 lockers). macOS and Windows can build core and daemon later; session lock crates target Linux first. |
| System libraries | Needed when you build GUI lock crates (`recallgate-lock-wayland`, `recallgate-lock-x11`). Pure `recallgate-core` tests need only a C toolchain and `pkg-config` on some setups. |

## Quick start

```bash
git clone https://github.com/kzndotsh/recallgate.git
cd recallgate
cargo test --workspace
```

Run the same checks CI runs:

```bash
make check
```

## System packages

Install these before building Wayland or X11 lock binaries. Names differ by distribution; CI uses the Debian/Ubuntu column.

| Purpose | Debian / Ubuntu | Fedora | Arch |
| --- | --- | --- | --- |
| Build | `build-essential`, `pkg-config` | `@development-tools`, `pkg-config` | `base-devel`, `pkgconf` |
| GTK 4 + session lock | `libgtk-4-dev`, `libgtk-4-layer-shell-dev` | `gtk4-devel`, `gtk4-layer-shell` | `gtk4`, `gtk4-layer-shell` |
| Wayland | `libwayland-dev`, `libxkbcommon-dev` | `wayland-devel`, `libxkbcommon-devel` | `wayland`, `libxkbcommon` |
| X11 (lock-x11) | `libx11-dev`, `libxrandr-dev` | `libX11-devel`, `libXrandr-devel` | `libx11`, `libxrandr` |
| SQLite (daemon store) | `libsqlite3-dev` | `sqlite-devel` | `sqlite` |

If your distribution ships `gtk4-layer-shell` older than 1.1.0, build [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell) from source and set `PKG_CONFIG_PATH` / `LD_LIBRARY_PATH` to that prefix. See the [gtk4-session-lock crate README](https://docs.rs/crate/gtk4-session-lock/latest).

### Example: Ubuntu 24.04

Noble has no `libgtk-4-layer-shell-dev`. X11 locker headers:

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config \
  libx11-dev libxrandr-dev \
  libsqlite3-dev
```

Wayland `--features ui` needs GTK 4 and `gtk4-layer-shell` ≥ 1.1. Use `nix develop`, or build [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell) from source on another distro that ships those packages.

## Everyday commands

| Command | Purpose |
| --- | --- |
| `cargo test --workspace` | Unit and integration tests |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint |
| `cargo fmt --all` | Format (use `--check` in CI) |
| `make check` | `fmt` + `clippy` + `test` + conventional commit subjects |
| `make check-commits` | Validate commits since merge-base with `origin/main` |

## Platform expectations

Wayland session lock uses `ext-session-lock-v1`. It works on Sway, Hyprland, niri, labwc, and similar compositors. GNOME (Mutter) and KDE (KWin) do not implement this protocol; the Wayland locker exits with `unsupported` there. See [platforms.md](platforms.md).

Manual lock testing uses a nested compositor so you do not freeze your real session:

- **Wayland:** nested Sway (documented in [plan.md](plan.md) live lanes).
- **X11:** Xephyr.

## Optional: Nix development shell

```bash
nix develop
cargo test --workspace
```

The flake `devShell` installs the same libraries as the table above and puts **`rust-toolchain.toml` on `PATH`** via [rust-overlay](https://github.com/oxalica/rust-overlay) (`fromRustupToolchainFile`). That matches rustup and CI (currently Rust 1.83.0 with `rustfmt` and `clippy`). Use it when you want pinned GTK/Wayland versions without matching distro packages. Merging does not require Nix; GitHub Actions uses apt on `ubuntu-24.04`.

Build the Wayland locker binary (needs GTK 4 and `gtk4-layer-shell` ≥ 1.1):

```bash
cargo build -p recallgate-lock-wayland --features ui
```

Run `recallgate-lock-wayland` or `recallgate-lock-x11` **alongside** `recallgate-daemon`. Both lockers stay in their event loop after unlock or abort so cooldown relock and a later `gate_lock` work. Do not treat process exit as the unlock path.

Hatch (paid abort): hold `Ctrl+Shift+Escape` for 2 seconds, type `ABORT`, press Enter. Escape and Alt+F4 do not unlock.

Build the X11 locker (needs X11 and RandR development libraries):

```bash
cargo build -p recallgate-lock-x11 --features x11
```

Run `recallgate-lock-x11` with `recallgate-daemon` on a pure X11 session (for example Xephyr).

Build and run the MCP adapter (stdio only; requires a running daemon):

```bash
cargo build -p recallgate-mcp
recallgate-daemon &
recallgate-mcp
```

Logs go to stderr; stdout is MCP JSON-RPC only.

## Continuous integration

`.github/workflows/ci.yml` runs `make ci` (format, clippy, tests) without `--features ui`. An `x11-check` job installs X11 headers and `cargo check`/`clippy`s `recallgate-lock-x11 --features x11`. Ubuntu 24.04 has no `libgtk-4-layer-shell-dev`, and `gtk4` 0.10 pulls `cfg-expr` that needs Cargo edition2024, so `--features ui` is not a CI job. Use `nix develop` for a Wayland UI compile. Commit subjects are checked by `conventional-commits.yml`. Locally, run `make check` before push to run both.

## Related docs

- [Agent guide](../AGENTS.md)
- [Contributing](../CONTRIBUTING.md)
- [Domain types](domain.md)
- [Implementation plan](plan.md)
- [Platforms](platforms.md)

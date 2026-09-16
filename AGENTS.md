# Agent guide

Recall Gate freezes the desktop until the user answers one MCQ. Agents may enqueue prompts or start a freeze. They must not unlock the session.

## Read first

| Doc | Use when |
| --- | --- |
| [docs/domain.md](docs/domain.md) | Types, `GatePhase`, illegal states. Names there are canonical. |
| [docs/protocol.md](docs/protocol.md) | Daemon RPC and MCP methods. |
| [docs/plan.md](docs/plan.md) | PR order, crate boundaries, verification lanes. |
| [docs/platforms.md](docs/platforms.md) | Wayland session lock vs X11 grab, compositor support. |
| [CONTRIBUTING.md](CONTRIBUTING.md) | PR scope and product rules. |

## Commands

Setup is in [docs/developing.md](docs/developing.md). After dependencies are installed:

```bash
make check          # fmt, clippy -D warnings, tests (same as CI)
cargo test -p recallgate-core
```

Optional Nix: `nix develop` then the same `cargo` / `make` commands.

## Architecture (do not blur)

- **`recallgate-core`**: domain and scheduling. No windowing.
- **`recallgate-daemon`**: sole SQLite writer; Unix socket JSON-RPC at `$XDG_RUNTIME_DIR/recallgate.sock`.
- **Lock binaries** (`lock-wayland`, `lock-x11`): render UI only; talk RPC, never open the DB.
- **`recallgate-mcp`**: stdio MCP; tools mirror RPC. No `gate_unlock`.

Implement in plan order: `pr-core` → `pr-store` → `pr-daemon` → (`pr-wayland` ∥ `pr-x11`) → `pr-mcp`.

## Hard rules

- Never add `gate_unlock` to RPC or MCP.
- Never call AnkiConnect from the freeze path.
- Wrong MCQ default: flash correct answer, log `Again`, then unlock (see [docs/product.md](docs/product.md)).
- Wayland: `ext-session-lock-v1` via GTK session lock, not layer-shell overlay as the product lock.
- GNOME/KDE Wayland: no session lock protocol; locker reports `unsupported`, daemon/MCP still work.

## Testing

- Unit work: `cargo test` for the crate you touch.
- Lock UI: nested Sway (Wayland) or Xephyr (X11). Do not drive the real session lock on the host without operator consent.
- `docs/plan.md` requires live and perf evidence for merge-ready PRs, not tests alone.

## Cloud Agent environment

There is no committed `.cursor/environment.json` yet. CI uses Ubuntu 24.04 and `make check`. Full GTK/Wayland deps are documented for lock crates; core-only work needs only the Rust toolchain.

`docs/plan.md` references `pstack/` playbooks on `origin/main`. That tree is not in this repository unless added separately. Use [docs/plan.md](docs/plan.md) and this file for agent workflow here.

## When unsure

Prefer [docs/domain.md](docs/domain.md) over inventing new type names. Product behavior questions belong to the human operator, not a new RPC.

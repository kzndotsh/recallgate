# Recall Gate

Recall Gate locks every monitor until you answer one question with four choices. It sits between you and the desktop you already have, so review is not something you can postpone for “after this tab.”

Pick the right number (or click the row) and the session continues. Pick wrong and it still continues, after it shows the right answer. There is no skip. Abort exists, but it is deliberately worse than answering: hold Ctrl+Shift+Escape, type `ABORT`, then you get about a minute before the lock comes back.

Other programs (including chat agents) can add cards or start a lock. They cannot unlock.

Cards are stored on this computer. Importing an external deck is not in v0.

On Wayland (Sway and similar) the lock is a real session lock: if the lock program crashes, the screens stay blank until you start it again or switch to a TTY. On X11 it is only a grab. If that program crashes, the session comes back.

## Build and run

Toolchain, distro packages, and `make check` are in [docs/developing.md](docs/developing.md). Nix is optional (`nix develop`).

How we take PRs: [CONTRIBUTING.md](CONTRIBUTING.md).

## Docs

- [Why this exists](docs/product.md)
- [How to develop](docs/developing.md)
- [What the words mean](docs/domain.md)
- [Daemon and agent API](docs/protocol.md)
- [Wayland vs X11](docs/platforms.md)
- [Build order and live tests](docs/plan.md)
- [Notes for coding agents](AGENTS.md)

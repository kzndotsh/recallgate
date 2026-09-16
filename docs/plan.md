# Recall Gate plan

Recall Gate taxes continuing a desktop session with one multiple-choice card. The operator and later owners run this checklist. A freeze ends only by a rating or a paid abort. Stack order is `pr-core`, `pr-store`, `pr-daemon`, then `pr-wayland` and `pr-x11` in parallel, then `pr-mcp`.

## How to read this

One box is one unit of work. Every box names the evidence that checks it. A nested box is a sub-step of the box above it. Check a box only when its evidence exists, a file, a log line, a screenshot, a test run, or a SHA. The body is a how-to. The appendices explain and record.

The program runs `pstack/skills/poteto-mode/playbooks/autopilot-stack.md`. The operator lands every PR. Owners stop at merge-ready. `pr-wayland` and `pr-x11` wait for the operator in chat.

Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

## Program checklist

### Arm the program

- [ ] State the protocol and this plan to the operator, then stop. Start execution only on the operator's explicit go.
- [ ] On the operator's go, arm a `/goal` with this exact text. "`docs/plan.md`. PRs `pr-core` `pr-store` `pr-daemon` `pr-wayland` `pr-x11` `pr-mcp`. A PR is verified only when its unit, live, and perf boxes are all checked. The operator lands. Done when `pr-mcp` is on `main` and `gate_lock` freezes a nested compositor with one card."
- [ ] Read these from trunk at program start. Re-read them at every tick.
  - [ ] `git show origin/main:pstack/skills/poteto-mode/playbooks/autopilot-stack.md`
  - [ ] `git show origin/main:pstack/skills/swarm/SKILL.md`
  - [ ] `git show origin/main:pstack/skills/control-cli/SKILL.md`
  - [ ] `git show origin/main:pstack/skills/poteto-mode/playbooks/opening-a-pr.md`
  - [ ] `git show origin/main:pstack/skills/how/SKILL.md`
  - [ ] `git show origin/main:pstack/skills/principle-model-the-domain/SKILL.md`
- [ ] Arm the 30-minute audit tick. In a local session, a real terminal `/loop`. In a cloud root, a cloud-sleeper wake chain. Never leave the cadence to memory.
- [ ] Use this tick prompt, verbatim. "Re-read the execution playbook from trunk and the armed /goal. Audit the operation against both and fix drift in this tick. Probe every active lane and judge progress by side effects only. Stand down a stuck lane and dispatch its replacement now. Then post a status message to the operator in chat, whether or not anything changed, with the queue table of PR, owner, state, and head SHA, the verdicts since the last tick, what merged, open operator gates, and blockers."
- [ ] On the operator's hold or stand-down, send every owner a zero-writes order at once.

### Spawn owners

- [ ] Spawn one owner per PR with the full lifecycle the execution playbook names.
- [ ] Follow this dependency graph. Start dependent work only after its parent merges, or base it on the parent branch when the execution playbook stacks.
  - [ ] `pr-core` is first. It branches from `main`.
  - [ ] `pr-store` after `pr-core`.
  - [ ] `pr-daemon` after `pr-store`.
  - [ ] `pr-wayland` and `pr-x11` after `pr-daemon`. Those two are independent of each other.
  - [ ] `pr-mcp` after `pr-daemon`. It must not import lock crates.
- [ ] Hold the file boundaries. `pr-core` touches only `crates/core/**` plus workspace `Cargo.toml` and `flake.nix`. `pr-store` touches only `crates/core/**`. `pr-daemon` touches only `crates/daemon/**` and `docs/protocol.md`. `pr-wayland` touches only `crates/lock-wayland/**`. `pr-x11` touches only `crates/lock-x11/**`. `pr-mcp` touches only `crates/mcp/**`.
- [ ] Hold the review gate. `pr-wayland` and `pr-x11` change an interaction. They wait for the operator's review in chat with screenshots and a video before merge.

### PR mechanics, for every PR

- [ ] Resolve the forge once. Default to `gh`; if `command -v origin` succeeds and Origin can resolve the repository, use `origin pr` for every PR operation. Record any fallback to `gh`. Never require `gt`.
- [ ] Open the PR ready, never draft, with `origin pr create --status open --base <base-branch>` or `gh pr create --base <base-branch>` according to the resolved forge. A stack child targets its parent branch.
- [ ] Run the repo's lint and typecheck once before the PR-facing push. Push with hooks on.
- [ ] Run `/deslop` before each commit and `/no-comments` before review.
- [ ] Triage every Bugbot and security-reviewer comment per `../references/bugbot-triage.md`.
- [ ] Rebase onto current trunk before babysit and again before the merge-ready report.

### Verdict and merge, for every PR

- [ ] At the merge-ready head SHA, run the swarm per `pstack/skills/swarm/SKILL.md`. One gates lane. The ten live lanes from the PR's **Verify, live** block. The perf lane from its **Verify, perf** block. One audit lane that reads the diff and the receipts and distrusts the PR body.
- [ ] Clean only when every lane is `PASS`. Findings go back to the owner. A new head gets a fresh swarm and a fresh verdict.
- [ ] The root appends the PR to the base-branch stack. The operator lands bottom-up. After rebase, compare `git patch-id` to the verdict SHA per `playbooks/shipping.md`. A changed patch-id voids the verdict.

### Boot recipe, for every live lane

Each live lane runs on its own cloud VM at the PR head. Drive through `control-cli` from `cursor-team-kit`. Native lock windows have no control-ui skill. See Appendix C.

- [ ] `git fetch origin <head-branch> && git checkout <head SHA>`.
- [ ] Install the toolchain and system deps per `docs/developing.md` (or `nix develop` if using the optional flake). Wait until `cargo --version` prints.
- [ ] Deliver input only through `control-cli` commands. Read `cargo test` stdout and journal logs. Do not type into a lock surface from the agent except where a lane names `wtype` or `ydotool`.
- [ ] Save every screenshot to `/tmp/swarm-<pr-id>/worker-<n>/<slug>.png` and return the paths with the report.

## Land core types (pr-core)

**Depends on.** None.

**Files.**

- [ ] Create `Cargo.toml`.
- [ ] Create `crates/core/Cargo.toml`.
- [ ] Create `crates/core/src/lib.rs`.
- [ ] Create `crates/core/src/ids.rs`.
- [ ] Create `crates/core/src/prompt.rs`.
- [ ] Create `crates/core/src/session.rs`.
- [ ] Create `crates/core/src/schedule.rs`.
- [ ] Create `flake.nix`.
- [ ] Create `.gitignore`.

**Build.**

- [ ] Add newtypes `ItemId`, `GateId` in `crates/core/src/ids.rs`.
- [ ] Add `Item`, `Schedule`, `Rating`, `GatePhase`, `Abort`, `ReviewLog` per [domain.md](domain.md). An abort cannot construct a `ReviewLog`.

**You see.**

- [ ] `cargo test -p recallgate-core` exits 0 and prints `test result: ok`.

**Verify, unit.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] `crates/core/src/session.rs` rejects `Unlocked` with an open session. Run `cargo test -p recallgate-core session`.

**Verify, live.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked. Ten lanes on `grok-4.6-fast-xhigh` at the PR head, per the boot recipe.

- [ ] Lane 1. Regression lane against trunk. Run `cargo test -p recallgate-core` at trunk and head. If trunk lacks the crate, record that and gate that the new tests pass and the process prints `test result: ok`. Save `pr-core-lane-1.png`. Pass when head stdout contains `test result: ok`.
- [ ] Lane 2. Compile with `cargo test -p recallgate-core --offline` after `cargo fetch`. Save `pr-core-lane-2.png`. Pass when the command exits 0.
- [ ] Lane 3. `clippy` on `crates/core` with `-D warnings`. Save `pr-core-lane-3.png`. Pass when clippy exits 0.
- [ ] Lane 4. Construct an `Mcq` prompt and map a wrong index to `Rating::Again`. Save `pr-core-lane-4.png`. Pass when that test name is in stdout.
- [ ] Lane 5. Construct a correct index mapped to `Rating::Good`. Save `pr-core-lane-5.png`. Pass when that test name is in stdout.
- [ ] Lane 6. Attempt `GatePhase::Idle` while a session id is still held. Save `pr-core-lane-6.png`. Pass when the test asserts the constructor returns `Err`.
- [ ] Lane 7. `Abort` type has no `ReviewLog` field. Save `pr-core-lane-7.png`. Pass when `cargo test` covers `abort_is_not_a_review`.
- [ ] Lane 8. Suspended queue cannot become a lock prompt. Save `pr-core-lane-8.png`. Pass when `due_excludes_suspended` passes.
- [ ] Lane 9. `make check` on the Ubuntu CI-equivalent image (see `.github/workflows/ci.yml`), or `nix build .#recallgate-core` when using Nix. Save `pr-core-lane-9.png`. Pass when the chosen command exits 0.
- [ ] Lane 10. `cargo fmt --check`. Save `pr-core-lane-10.png`. Pass when rustfmt reports no diffs.

**Verify, perf.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Metric. Wall time of `cargo test -p recallgate-core` at trunk and head. If trunk lacks the crate, also time the new test binary and the wait until stdout prints `test result: ok`.
- [ ] Probe. `/usr/bin/time -f %e cargo test -p recallgate-core` at trunk then head, then again interleaved once.
- [ ] Baseline. Record the trunk seconds first. If trunk has no crate, record `absent` and use only the head absolute budget.
- [ ] Rule. Head `cargo test -p recallgate-core` must finish in under 5 seconds on the lane VM. Fail at 5.000 or above. Do not ratio unlike trunk and head when trunk is absent.

**Review gate.** None. pr-core is not review-gated.

**Merge.**

- [ ] Root's clean verdict at the exact head SHA.
- [ ] Bugbot triage done.
- [ ] Rebased onto current trunk after the verdict, patch-id unchanged.
- [ ] The root appends `pr-core` to the stack. The operator lands it on `main`.

## Persist cards and sessions (pr-store)

**Depends on.** pr-core.

**Files.**

- [ ] Create `crates/core/src/store.rs`.
- [ ] Edit `crates/core/src/lib.rs`.
- [ ] Create `crates/core/tests/store_roundtrip.rs`.

**Build.**

- [ ] Add SQLite persistence for `Item`, schedule snapshots, `GatePhase`, `Abort`, and `ReviewLog` in `crates/core/src/store.rs`. Crash with `Locked` still on disk must reload as `Locked`.

**You see.**

- [ ] `cargo test -p recallgate-core store_roundtrip` prints `test result: ok` and a second process reads the same `GateId`.

**Verify, unit.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] `crates/core/tests/store_roundtrip.rs` reopens the file after drop. Run `cargo test -p recallgate-core store_roundtrip`.

**Verify, live.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked. Ten lanes on `grok-4.6-fast-xhigh` at the PR head, per the boot recipe.

- [ ] Lane 1. Regression lane against trunk. Run `cargo test -p recallgate-core` at trunk and head. If trunk lacks persistence, record that and gate reload of a locked row plus `test result: ok`. Save `pr-store-lane-1.png`. Pass when head includes `store_roundtrip` ok.
- [ ] Lane 2. Kill the test process after `begin_lock` then reopen. Save `pr-store-lane-2.png`. Pass when the reopened phase is `Locked`.
- [ ] Lane 3. Write an `Abort` row and assert `ReviewLog` count is unchanged. Save `pr-store-lane-3.png`. Pass when that assertion is in the test stdout.
- [ ] Lane 4. Two `Locked` inserts in one file fail. Save `pr-store-lane-4.png`. Pass when the second insert returns `Err`.
- [ ] Lane 5. Import a fixture JSON prompt and list it as due. Save `pr-store-lane-5.png`. Pass when due count is 1.
- [ ] Lane 6. Suspended card is absent from due. Save `pr-store-lane-6.png`. Pass when due count is 0 for that card.
- [ ] Lane 7. Scheduler `memory` snapshot roundtrips without loss (for example stability and difficulty if using FSRS defaults). Save `pr-store-lane-7.png`. Pass when equality holds.
- [ ] Lane 8. `clippy -D warnings` on `crates/core`. Save `pr-store-lane-8.png`. Pass when clippy exits 0.
- [ ] Lane 9. Corrupt the db header and open. Save `pr-store-lane-9.png`. Pass when open returns a typed error, not a panic.
- [ ] Lane 10. Concurrent writers are not in this crate. Document single-writer in the test name `store_is_single_writer`. Save `pr-store-lane-10.png`. Pass when that test exists and passes.

**Verify, perf.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Metric. Time to insert 1000 prompts and read due at trunk and head. If trunk lacks store, also time that insert loop and the wait until due returns 1000.
- [ ] Probe. `cargo test -p recallgate-core persist_1000 -- --nocapture` at trunk and head, interleaved.
- [ ] Baseline. Record trunk milliseconds first, or `absent`.
- [ ] Rule. Head insert of 1000 prompts plus due read must finish in under 500 ms. Fail at 500 ms or above.

**Review gate.** None. pr-store is not review-gated.

**Merge.**

- [ ] Root's clean verdict at the exact head SHA.
- [ ] Bugbot triage done.
- [ ] Rebased onto current trunk after the verdict, patch-id unchanged.
- [ ] The root appends `pr-store`. The operator lands it.

## Speak daemon rpc (pr-daemon)

**Depends on.** pr-store.

**Files.**

- [ ] Create `crates/daemon/Cargo.toml`.
- [ ] Create `crates/daemon/src/main.rs`.
- [ ] Create `crates/daemon/src/rpc.rs`.
- [ ] Edit `docs/protocol.md` only to match shipped methods.

**Build.**

- [ ] Add a Unix socket JSON-RPC server in `crates/daemon/src/rpc.rs` with `gate_lock`, `gate_status`, `gate_push_prompt`, and `gate_due`. There is no `gate_unlock` method.

**You see.**

- [ ] `recallgate-daemon` listens on `$XDG_RUNTIME_DIR/recallgate.sock` and `gate_status` returns `{"phase":"idle"}`.

**Verify, unit.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] `crates/daemon/src/rpc.rs` rejects unknown methods. Run `cargo test -p recallgate-daemon rpc`.

**Verify, live.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked. Ten lanes on `grok-4.6-fast-xhigh` at the PR head, per the boot recipe.

- [ ] Lane 1. Regression lane against trunk. Run a status RPC at trunk and head. If trunk has no daemon, record that and gate that `gate_status` returns idle and the process stays up. Save `pr-daemon-lane-1.png`. Pass when the JSON `phase` is `idle`.
- [ ] Lane 2. Call `gate_lock` with a fixture prompt without a lock backend. Save `pr-daemon-lane-2.png`. Pass when the error name is `no_lock_backend` and phase stays idle.
- [ ] Lane 3. Call `gate_unlock`. Save `pr-daemon-lane-3.png`. Pass when the response is method-not-found.
- [ ] Lane 4. `gate_push_prompt` then `gate_due`. Save `pr-daemon-lane-4.png`. Pass when due length is 1.
- [ ] Lane 5. Second daemon on the same socket fails. Save `pr-daemon-lane-5.png`. Pass when the second process exits nonzero.
- [ ] Lane 6. SIGTERM leaves the db closed cleanly. Save `pr-daemon-lane-6.png`. Pass when reopen works.
- [ ] Lane 7. Malformed JSON line. Save `pr-daemon-lane-7.png`. Pass when the next well-formed call still works.
- [ ] Lane 8. `clippy -D warnings` on `crates/daemon`. Save `pr-daemon-lane-8.png`. Pass when clippy exits 0.
- [ ] Lane 9. `control-cli` script calls `gate_status` 20 times. Save `pr-daemon-lane-9.png`. Pass when all 20 return idle.
- [ ] Lane 10. Socket path follows `XDG_RUNTIME_DIR`. Save `pr-daemon-lane-10.png`. Pass when `lsof` shows that path.

**Verify, perf.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Metric. Round-trip ms of `gate_status` at trunk and head. If trunk has no daemon, also time that round trip and the wait until JSON prints.
- [ ] Probe. 100 serial `gate_status` calls at trunk and head, interleaved.
- [ ] Baseline. Record trunk p99 ms first, or `absent`.
- [ ] Rule. Head p99 `gate_status` must be under 20 ms. Fail at 20 ms or above.

**Review gate.** None. pr-daemon is not review-gated.

**Merge.**

- [ ] Root's clean verdict at the exact head SHA.
- [ ] Bugbot triage done.
- [ ] Rebased onto current trunk after the verdict, patch-id unchanged.
- [ ] The root appends `pr-daemon`. The operator lands it.

## Lock the Wayland session (pr-wayland)

**Depends on.** pr-daemon.

**Files.**

- [ ] Create `crates/lock-wayland/Cargo.toml`.
- [ ] Create `crates/lock-wayland/src/main.rs`.
- [ ] Create `crates/lock-wayland/src/lock.rs`.

**Build.**

- [ ] Add `ext-session-lock-v1` via `gtk4-session-lock` in `crates/lock-wayland/src/lock.rs`. One window per output. Unlock only after `core` returns a rating or a completed `Abort`.

**You see.**

- [ ] Nested Sway shows one MCQ and ignores other clients until a choice key. After Good, the nested session is usable.

**Verify, unit.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] `crates/lock-wayland` maps keys 1 to 4 onto choice indices. Run `cargo test -p recallgate-lock-wayland keys`.

**Verify, live.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked. Ten lanes on `grok-4.6-fast-xhigh` at the PR head, per the boot recipe.

- [ ] Lane 1. Regression lane against trunk. Run the nested Sway lock at trunk and head. If trunk has no locker, record that and gate that the lock surface appears and a correct key restores the nested desktop. Save `pr-wayland-lane-1.png`. Pass when the nested desktop is visible after unlock.
- [ ] Lane 2. Wrong key flashes the answer then unlocks. Save `pr-wayland-lane-2.png`. Pass when the session is usable and `ReviewLog` has `Again`.
- [ ] Lane 3. Hatch chord plus hold plus typed confirm starts a cooldown. Save `pr-wayland-lane-3.png`. Pass when `gate_status` is `cooldown` and no `ReviewLog` row was added.
- [ ] Lane 4. Two outputs get two lock surfaces. Save `pr-wayland-lane-4.png`. Pass when both screenshots show the same stem.
- [ ] Lane 5. Kill the lock client after `locked`. Save `pr-wayland-lane-5.png`. Pass when the nested compositor stays blank until a second lock client attaches.
- [ ] Lane 6. Overlay-only GTK window without session lock is not this binary. Save `pr-wayland-lane-6.png`. Pass when `WAYLAND_DISPLAY` nested compositor `protocol` log contains `ext_session_lock`.
- [ ] Lane 7. `Escape` and `Alt+F4` do not unlock. Save `pr-wayland-lane-7.png`. Pass when phase stays `locked`.
- [ ] Lane 8. `clippy -D warnings`. Save `pr-wayland-lane-8.png`. Pass when clippy exits 0.
- [ ] Lane 9. Mutter or a compositor without the protocol. Save `pr-wayland-lane-9.png`. Pass when the binary exits with `unsupported` and does not grab input.
- [ ] Lane 10. Cooldown timer expires and relocks. Save `pr-wayland-lane-10.png`. Pass when a second lock surface appears without a new `gate_lock` from the user.

**Verify, perf.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Metric. Time from `gate_lock` to first lock frame presented, at trunk and head. If trunk has no locker, also time that wait and the wait until the nested output is opaque.
- [ ] Probe. `WAYLAND_DEBUG=1` plus a stopwatch from RPC send to the nested `locked` event, trunk then head, interleaved.
- [ ] Baseline. Record trunk milliseconds first, or `absent`.
- [ ] Rule. Head time to opaque lock frames must be under 300 ms after the compositor is already running. Fail at 300 ms or above.

**Review gate.** The operator reviews before merge.

- [ ] Copy lane 1 screenshots into `docs/media/pr-wayland-review-lock.png`.
- [ ] Record a 30 to 60 second video of the change on a lane VM. Save it as `docs/media/pr-wayland-review.mp4`.
- [ ] Post the screenshots and the video in chat. Stop at merge-ready. Wait for the operator's click.

**Merge.**

- [ ] Root's clean verdict at the exact head SHA.
- [ ] Bugbot triage done.
- [ ] Rebased onto current trunk after the verdict, patch-id unchanged.
- [ ] The root appends `pr-wayland`. The operator lands after the review click.

## Grab the X11 session (pr-x11)

**Depends on.** pr-daemon.

**Files.**

- [ ] Create `crates/lock-x11/Cargo.toml`.
- [ ] Create `crates/lock-x11/src/main.rs`.
- [ ] Create `crates/lock-x11/src/grab.rs`.

**Build.**

- [ ] Add root `XGrabKeyboard` and `XGrabPointer`, override-redirect windows per RandR output, and composite overlay covering in `crates/lock-x11/src/grab.rs`. Capability advertised is `x11_grab`, not `session_lock`.

**You see.**

- [ ] Xephyr shows one MCQ. Other Xephyr clients do not receive keys until a choice. Crash of the locker returns the Xephyr session.

**Verify, unit.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Grab failure is a typed error. Run `cargo test -p recallgate-lock-x11 grab_err`.

**Verify, live.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked. Ten lanes on `grok-4.6-fast-xhigh` at the PR head, per the boot recipe.

- [ ] Lane 1. Regression lane against trunk. Run the Xephyr grab at trunk and head. If trunk has no X11 locker, record that and gate that a correct key restores Xephyr input. Save `pr-x11-lane-1.png`. Pass when `xdotool` can type into a nested xterm after unlock.
- [ ] Lane 2. Wrong key then unlock with `Again` logged. Save `pr-x11-lane-2.png`. Pass when `ReviewLog` has `Again`.
- [ ] Lane 3. Hatch produces `cooldown` with no review row. Save `pr-x11-lane-3.png`. Pass when `gate_status` is `cooldown`.
- [ ] Lane 4. Kill the locker while grabbed. Save `pr-x11-lane-4.png`. Pass when Xephyr accepts keys again without a second client.
- [ ] Lane 5. Compositor in Xephyr. Save `pr-x11-lane-5.png`. Pass when the quiz still receives keys with picom running.
- [ ] Lane 6. Grab already held by a nested menu. Save `pr-x11-lane-6.png`. Pass when the binary reports `grab_busy` and does not claim `session_lock`.
- [ ] Lane 7. `gate_status` capability field is `x11_grab`. Save `pr-x11-lane-7.png`. Pass when JSON contains `x11_grab`.
- [ ] Lane 8. `clippy -D warnings`. Save `pr-x11-lane-8.png`. Pass when clippy exits 0.
- [ ] Lane 9. Two RandR outputs in Xephyr. Save `pr-x11-lane-9.png`. Pass when both show the stem.
- [ ] Lane 10. `FORCE_GRAB` unmap is absent from default flags. Save `pr-x11-lane-10.png`. Pass when `--help` has no force-unmap default on.

**Verify, perf.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Metric. Time from `gate_lock` to successful grab, at trunk and head. If trunk has no X11 locker, also time that wait and the wait until the overlay is mapped.
- [ ] Probe. Stopwatch around `XGrabKeyboard` success, trunk then head, interleaved.
- [ ] Baseline. Record trunk milliseconds first, or `absent`.
- [ ] Rule. Head grab plus map must finish in under 200 ms with Xephyr already running. Fail at 200 ms or above.

**Review gate.** The operator reviews before merge.

- [ ] Copy lane 1 screenshots into `docs/media/pr-x11-review-lock.png`.
- [ ] Record a 30 to 60 second video of the change on a lane VM. Save it as `docs/media/pr-x11-review.mp4`.
- [ ] Post the screenshots and the video in chat. Stop at merge-ready. Wait for the operator's click.

**Merge.**

- [ ] Root's clean verdict at the exact head SHA.
- [ ] Bugbot triage done.
- [ ] Rebased onto current trunk after the verdict, patch-id unchanged.
- [ ] The root appends `pr-x11`. The operator lands after the review click.

## Expose MCP tools (pr-mcp)

**Depends on.** pr-daemon.

**Files.**

- [ ] Create `crates/mcp/Cargo.toml`.
- [ ] Create `crates/mcp/src/main.rs`.
- [ ] Create `crates/mcp/src/tools.rs`.

**Build.**

- [ ] Add stdio MCP in `crates/mcp/src/main.rs` that calls the daemon. Tools are `gate_lock`, `gate_push_prompt`, `gate_status`, and `gate_due`. Do not add `gate_unlock`.

**You see.**

- [ ] `recallgate-mcp` lists those four tools. A `tools/call` of `gate_unlock` is unknown.

**Verify, unit.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Tool schema omits unlock. Run `cargo test -p recallgate-mcp tools`.

**Verify, live.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked. Ten lanes on `grok-4.6-fast-xhigh` at the PR head, per the boot recipe.

- [ ] Lane 1. Regression lane against trunk. Run `tools/list` at trunk and head. If trunk has no MCP binary, record that and gate that four tools list and `gate_status` returns JSON. Save `pr-mcp-lane-1.png`. Pass when `tools/list` names `gate_lock`.
- [ ] Lane 2. `gate_push_prompt` through MCP then `gate_due`. Save `pr-mcp-lane-2.png`. Pass when due length is 1.
- [ ] Lane 3. Call a tool named `gate_unlock`. Save `pr-mcp-lane-3.png`. Pass when MCP returns unknown tool.
- [ ] Lane 4. Daemon down. Save `pr-mcp-lane-4.png`. Pass when the tool result is a structured error, not a hang past 2 seconds.
- [ ] Lane 5. Logs go to stderr only. Save `pr-mcp-lane-5.png`. Pass when stdout is JSON-RPC lines only.
- [ ] Lane 6. `clippy -D warnings`. Save `pr-mcp-lane-6.png`. Pass when clippy exits 0.
- [ ] Lane 7. OpenClaw-shaped stdio handshake initialize then list. Save `pr-mcp-lane-7.png`. Pass when `protocolVersion` is negotiated.
- [ ] Lane 8. Resource `recallgate://session`. Save `pr-mcp-lane-8.png`. Pass when read returns phase.
- [ ] Lane 9. HTTP transport is absent in this PR. Save `pr-mcp-lane-9.png`. Pass when `--help` has no `--http` flag.
- [ ] Lane 10. `cargo fmt --check` on `crates/mcp`. Save `pr-mcp-lane-10.png`. Pass when rustfmt reports no diffs.

**Verify, perf.** Tests alone are not sufficient verification. A PR is verified only when its unit, live, and perf boxes are all checked.

- [ ] Metric. `tools/list` round trip ms at trunk and head. If trunk has no MCP, also time that list and the wait until the result JSON prints.
- [ ] Probe. 50 `tools/list` calls at trunk and head, interleaved.
- [ ] Baseline. Record trunk p99 ms first, or `absent`.
- [ ] Rule. Head p99 `tools/list` must be under 50 ms with the daemon already up. Fail at 50 ms or above.

**Review gate.** None. pr-mcp is not review-gated.

**Merge.**

- [ ] Root's clean verdict at the exact head SHA.
- [ ] Bugbot triage done.
- [ ] Rebased onto current trunk after the verdict, patch-id unchanged.
- [ ] The root appends `pr-mcp`. The operator lands it.

## Close the program

- [ ] Every box above is checked with its evidence.
- [ ] Reply to the operator with the report the execution playbook names.

## Appendix A. Prototype evidence

No prototype branch or SHA exists. The operator forbade coding in this pass.

Unproven. Nested Sway `ext-session-lock-v1` with GTK4 `gtk4-session-lock` on this machine. Xephyr plus picom grab covering. Cooldown timer versus compositor idle. Hatch chord versus IME. Crash restore on Sway without a `--locked` bind. MCP stdio with a live OpenClaw or Hermes client.

Product calls that still need the operator. Whether a wrong MCQ unlocks after the flash. Whether Windows and macOS backends enter the stack after `pr-mcp`. Whether HTTP MCP is wanted for SillyTavern in a later PR.

## Appendix B. Alternatives rejected

Electron or Tauri as the lock client. Those toolkits take `xdg_toplevel` and cannot own a lock surface.

AnkiConnect as the lock path. Anki must be running. HTML on a lock surface. Two schedulers.

Layer-shell overlay as the Wayland product. Crash leaks. Other outputs stay live if one surface is mapped.

Card on PAM or swaylock. Mixes auth with study.

Forking swaylock. Cairo password UI is a poor MCQ host.

Default `FORCE_GRAB` on X11. Unmapping clients to steal a grab breaks fullscreen games.

## Appendix C. Risks

No `control-ui` for GTK lock windows. Live lanes use nested Sway or Xephyr plus screenshots. Owners watch that the VM has a compositor. Lands in `pr-wayland` and `pr-x11`.

This repo has no `origin/main` and no vendored `pstack`. `git show origin/main:pstack/...` fails until a remote exists. Arming the program includes adding the remote or copying playbook paths.

Wayland crash stays locked. Owners must document TTY and a compositor `--locked` bind in the `pr-wayland` PR body.

X11 crash fails open. Users may think the product is broken when it is following the X server. Lands in `pr-x11`.

Single SQLite writer. The daemon is the only process that opens the db. Lock binaries talk RPC only. Lands in `pr-store` and `pr-daemon`.

## Appendix D. Links and reading list

Read `docs/product.md`, `docs/domain.md`, `docs/protocol.md`, and `docs/platforms.md` before any PR.

`pr-wayland` and `pr-x11` run `pstack/skills/how/SKILL.md` on the lock crate before editing. Those two also run `pstack/skills/interrogate/SKILL.md` if the grab versus lock debate reopens.

Start a `decisions.tsv` trail per `pstack/skills/show-me-your-work/SKILL.md` within 15 minutes of each owner spawn.

Protocol pages used in the prior investigation. [ext-session-lock-v1](https://wayland.app/protocols/ext-session-lock-v1). [GTK4 session lock](https://wmww.github.io/gtk4-layer-shell/gtk4-layer-shell-GTK4-Session-Lock.html). [xsecurelock](https://github.com/google/xsecurelock). [Why X11 lockers cannot be secure](https://blog.martin-graesslin.com/blog/2015/01/why-screen-lockers-on-x11-cannot-be-secure/).

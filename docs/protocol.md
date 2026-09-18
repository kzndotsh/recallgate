# Protocol reference

The daemon is the only writer of the item database. Lock binaries and MCP are clients.

Transport for v0 is a Unix socket at `$XDG_RUNTIME_DIR/recallgate.sock`. Framing is newline-delimited JSON-RPC 2.0.

There is no `gate_unlock` method.

## Methods

### gate_lock

Starts a freeze if a lock backend is bound and [gate cadence](domain.md#gate-cadence-v0) allows it.

Params (object, all optional unless noted).

- `prompt_id` string. If omitted, the daemon `pick_item`s from the active deck
- `reason` string. `idle`, `manual`, or `rpc`

Result on success.

- `session_id` string
- `prompt` object with `stem` and `choices` array of four strings. The correct index is not in this object

Errors.

- `no_lock_backend` when no Wayland or X11 backend is running
- `already_locked`
- `empty_deck` when there is no non-suspended item to show
- `cadence_active` when `lock_interval` has not elapsed since the last unlock
- `grab_busy` when the lock helper reports it could not grab (X11 already-grabbed, or equivalent)
- `illegal_transition` when the store rejects the phase change

If the gate is already `locked` and a helper is bound, `gate_lock` re-sends the current item to the helper and returns the existing `session_id`. It does not return `already_locked` in that case. If the helper is missing, the error is `already_locked` or `no_lock_backend`.

`reason` is accepted (`idle`, `manual`, or `rpc`) and ignored for v0 scheduling.

The daemon does not persist `Locked` until the helper acks show (see [Helper show IPC](#helper-show-ipc)).

### gate_status

Result.

- `phase` string. `idle`, `locked`, or `cooldown`
- `capability` string. `session_lock`, `x11_grab`, or `none`
- `session_id` string or null
- `cooldown_until` RFC3339 string or null

### gate_push_prompt

Params.

- `stem` string
- `choices` array of four strings
- `correct_index` integer 0..3
- `external_ref` string, optional

Result.

- `prompt_id` string
- `card_id` string

In v0 both ids are the same [item id](domain.md#wire-mapping-v0). They may diverge in a future version if presentation snapshots split from deck items.

This method does not freeze the session.

### gate_due

Lists items that may appear on the lock surface in v0. Same set as non-suspended deck items. The name is historical. It does not mean per-item due times. See [domain.md](domain.md#picking-an-item).

Result.

- `prompt_ids` array of strings (`ItemId` values)

Lock clients call `gate_submit_choice` on the daemon socket after the user picks an answer. That method is for the lock binary only, not MCP.

### gate_submit_choice

Lock backend only. Params.

- `chosen_index` integer 0..3

Result.

- `correct` boolean

Errors.

- `not_locked` when the gate is not in `locked` phase

### gate_abort

Lock backend only. MCP does not list this tool. Params object may be empty or `{ "method": "hatch" }`. If `method` is omitted it is stored as `hatch`.

Effects: append an abort event, enter cooldown (`until = now + cooldown_duration_ms` from settings, default 60s). No `Answer` row.

Result.

- `cooldown_until` RFC3339 string

The locker must ungrab and stay running. The daemon timer later calls `finish_cooldown_begin_lock` with a new item and sends show again.

## Helper show IPC

Lock helpers listen on `$XDG_RUNTIME_DIR/recallgate-wayland.sock` or `recallgate-x11.sock`.

1. Daemon writes one JSON line: `stem`, `choices`, `correct_index`, `session_id`.
2. Helper maps windows and locks/grabs.
3. Helper writes one ack line `{ "ok": true }` or `{ "ok": false, "error": "grab_busy" | "unsupported" }`.
4. Daemon `begin_lock` only after `ok`. Timeout 2s with no ack is `no_lock_backend`.

## MCP

Binary name `recallgate-mcp`. Stdio only in the first MCP PR.

Tools `gate_lock`, `gate_push_prompt`, `gate_status`, and `gate_due` only. `gate_unlock`, `gate_submit_choice`, and `gate_abort` must not appear in `tools/list`.

Logs write to stderr. Stdout is JSON-RPC only.

Resource `recallgate://session` reads the same payload as `gate_status`.

## Lock backend process

The lock binary does not open SQLite. It connects to the daemon socket. It renders `stem` and `choices`. It sends `gate_submit_choice` or `gate_abort` after a completed hatch. The daemon returns whether to ungrab.

Wayland `ext-session-lock-v1` unlock must not quit the GTK process. Both lockers stay resident so cooldown relock and a later `gate_lock` can show again. Escape and Alt+F4 do not unlock.

Hatch: hold `Ctrl+Shift+Escape` for 2 seconds, type `ABORT`, press Enter.

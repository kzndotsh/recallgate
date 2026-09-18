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
- `grab_busy` on X11 when another client holds the grab

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

## MCP

Binary name `recallgate-mcp`. Stdio only in the first MCP PR.

Tools with the same names and schemas as the methods above. `gate_unlock` must not appear in `tools/list`.

Logs write to stderr. Stdout is JSON-RPC only.

Resource `recallgate://session` reads the same payload as `gate_status`.

## Lock backend process

The lock binary does not open SQLite. It connects to the socket. It renders `stem` and `choices`. It sends the chosen index or a completed abort sequence to the daemon. The daemon returns whether to call the platform unlock.

Wayland unlock is `unlock_and_destroy` on the session lock object. X11 unlock is ungrab plus unmap.

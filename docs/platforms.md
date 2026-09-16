# Platform backends

The daemon picks one backend at start.

1. If `WAYLAND_DISPLAY` is set and `ext-session-lock-v1` is advertised, use the Wayland locker.
2. Else if `DISPLAY` is set, use the X11 locker.
3. Else refuse to freeze. MCP and RPC still work. `capability` is `none`.

Do not fall back from a failed Wayland lock to X11 on the same host while a Wayland session is active.

## Wayland

Protocol `ext-session-lock-v1`. Library path is GTK4 plus `gtk4-session-lock`, which uses `gtk4-layer-shell` 1.1 or newer for the lock API.

One lock surface per `wl_output`. Assign unrealized GTK windows. Size matches configure. Crash after `locked` does not unlock. Recovery is a second lock client on that `WAYLAND_DISPLAY`, a compositor bind that still fires while locked, or a TTY.

Compositors that speak the protocol include Sway 1.11+, Hyprland, niri, and labwc. Mutter and KWin do not. On those, the binary exits `unsupported`.

Idle trigger is `swayidle` or an equivalent. Do not fight `swaylock` on the same lock event. Either replace it for those triggers or run after it unlocks.

## X11

There is no session lock object. The backend emulates a lock.

- Override-redirect fullscreen window on each RandR output
- Cover the composite overlay so a compositor cannot stack notifications above the quiz
- `XGrabKeyboard` and `XGrabPointer` on the root window

`capability` is `x11_grab`.

Crash releases the grab. That is X server behavior.

If the grab fails because another client holds it, return `grab_busy`. Do not unmap other clients by default.

`XQueryKeymap` and `/dev/input` can still be read by other programs. Do not document X11 as a security boundary.

Idle trigger is an X screensaver or idle helper such as `xss-lock`, not `swayidle`.

## Later hosts

Windows uses a topmost window plus `BlockInput` or low-level hooks. Ctrl+Alt+Del always unblocks.

macOS uses a `CGEvent` tap plus kiosk presentation options. Accessibility permission is required.

Those hosts are not in `docs/plan.md` PR ids. Add them as new PR sections when the operator extends the stack.

## Packaging notes

Nix flake plus `wrapGAppsHook4` for the Wayland locker. X11 locker needs xcb or x11rb and RandR.

Do not ship the Wayland locker as a Flatpak if the sandbox strips session lock.

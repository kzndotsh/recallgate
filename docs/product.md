# About the session tax

Recall Gate exists because spaced repetition fails when it is optional in the same minute you open a browser. The product puts one card between you and the session you already have.

The loop is a tax on continuing, not a quota to clear. Phone apps that block Instagram until you finish Anki miss the desktop case. You already live in the browser. An app gate never fires.

## What the user meets

A freeze covers every monitor. One stem. Four keys. A correct answer unlocks. A wrong answer shows the right choice, logs an incorrect answer (Again-style in the UI copy), and still unlocks. Piling more cards after a miss makes abort the rational move.

Abort exists and costs more than the card. An obscure chord, a hold, a typed confirm, then a cooldown (about a minute of use), then the freeze returns. There is no Skip control. There is no tray Unlock.

## What the product refuses

It will not mix login passwords with study. Muscle memory types the password and skips the card, or the card becomes the only unlock and bricks a meeting.

It will not take AnkiConnect on the freeze path. Anki may be closed. HTML templates do not belong on a lock surface. Two schedulers on one card poison AnkiWeb.

It will not let an agent unlock. OpenClaw, Hermes, and SillyTavern may start a freeze or enqueue a card. They do not get `gate_unlock`.

It will not claim X11 is as strong as Wayland session lock. X11 has no lock object. A grab is an emulation. Crash on X11 returns the session. Crash on Wayland stays locked until a second lock client or a TTY.

## Why the core is separate

Cards, ratings, abort, and the freeze phase live in a library with no windowing. Wayland, X11, and later Windows or macOS are drivers. MCP is another client of the same daemon. That split is how you move machines without rewriting memory.

## Wrong-answer policy

This explanation picks a default so implementers do not stall. A miss unlocks after the flash. Honest failure is not punished. Abort is punished. Change this only with a product call and a domain test.

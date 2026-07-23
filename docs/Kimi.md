# Kimi.md — Kimi Code session handoff

Task handoff file for cross-agent work (Claude ↔ Kimi). Companion to
`docs/STATUS.md` — that one tracks the project; this one tracks **what
this Kimi session was in the middle of** so another agent (or a fresh
Kimi session) can pick it up cold. **Last updated: 2026-07-23.**

## Current thread: Spotlight-style AI surfaces + iMac field test

Asked for: macOS-Spotlight-with-Apple-Intelligence UX on the GNOME
image — search from Super+Space, start a Lisa session from there.

### Landed on `main` (pushed)

- `848476a` — "Ask Lisa" lane in the launcher: every overview query ≥2
  chars gets an Ask Lisa result (promoted near top for question-like
  queries, last otherwise; calc keeps top slot). Enter →
  `org.lisa.Overlay1.UI.Summon(query)` → overlay opens with the prompt
  already submitted. New frontend-owned D-Bus surface
  `org.lisa.Overlay1.UI` (Summon/Hide/GetVisible) in
  `shell/overlay-extension/extension.js`; iface XML in
  `shell/overlay-extension/lib/iface.js`; pure ranking logic +
  7 new tests in `shell/launcher/lib/ranking.js` (`just shell-test`,
  48/48 green).
- `a7ea447` — macOS summon keys: **Super+Space = overview search**,
  **Super+Shift+Space = assistant overlay**, input switcher →
  Ctrl+Super+Space (`os/packages/lisa/10_lisa-shell.gschema.override`,
  overlay gschema, PLAN §5.7.1). **Plus the field-found GNOME 50 fix:**
  both extensions' `metadata.json` capped `shell-version` at 49 while
  the image ships GNOME 50.3 — they never loaded (state OUT OF DATE,
  `enable()` never ran). Now declare 50.
- `b90d08b` — inferenced session-bus story: `--dbus` flag; new per-user
  `os/packages/lisa/lisa-inferenced-dbus.service` (loopback 7778;
  system unit owns 7777 and can't reach the session bus under
  DynamicUser), enabled via `default.target.wants` in the PKGBUILD;
  bus-loss watchdog in `main.rs` exits the daemon when the D-Bus
  connection dies so systemd re-registers the name (was: name silently
  vanished on session restart).

### In flight

- **Release CI run 30045673893** (workflow_dispatch) building the
  image with the three commits; ~25 min/run. When it publishes, the
  iMac gets it via `lisa update` (sysupdate, releases are the source).
- **iMac state** (`192.168.1.7`, lisa/lisa, fresh image on the bigger
  disk): `qwen3-0.6b-instruct-q8` pulled into `/var/lib/lisa-models`
  (survives A/B). Real inference confirmed: `lisa ask` → real tokens
  (system daemon, llama engine). Overlay D-Bus path needs the new
  release.

### Next (the verification loop, in order)

1. `gh run watch 30045673893` → release published.
2. On the iMac: `sudo lisa update` (or wait for sysupdate.timer),
   reboot into slot B.
3. Verify: `gnome-extensions info lisa-overlay@lisa-os.org` → ACTIVE;
   `busctl --user list | grep Overlay` → `org.lisa.Overlay1.UI` owned
   by gnome-shell + `org.lisa.Inference1` owned by the user unit;
   `gdbus call --session -d org.lisa.Overlay1.UI -o /org/lisa/Overlay1/UI
   -m org.lisa.Overlay1.UI.Summon "haiku about entropy" "{}"` →
   GetVisible true, `lisa ledger` gains context.search + generate
   entries, real tokens stream (stub era is over).
4. Then the human click-through: Super+Space → type a question →
   Enter on Ask Lisa → overlay streams. First real "great AI
   experience" moment; note friction for the M4 polish list.

## iMac operational gotchas (cost real time)

- No `rsync`, no `which`, **no pacman db** (`/var/lib/pacman` absent —
  image is sysupdate-managed; don't try to pacman-install anything).
- `gsettings`/`gnome-extensions` over SSH need
  `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus`.
- Re-login for Shell changes: `sudo systemctl restart gdm` (autologin
  is on; plain logout parks at the greeter). **`loginctl
  terminate-session` with an empty arg kills YOUR OWN ssh session** —
  always pass the explicit id (`1` = seat0 graphical).
- macOS `tar` injects `._*` AppleDouble files; they break
  `glib-compile-schemas`. Use `COPYFILE_DISABLE=1 tar` when pushing
  files, and `find … -name "._*" -delete` on the target.
- Hand-syncing to `/usr/share/lisa/shell/` works for same-day tests
  but is throwaway — ship fixes via the release channel instead.
- `lisa models pull --blake3 <hash> <url> <name>` — no catalog-name
  shortcut; the pinned tuple lives in `models/catalog/catalog.toml`.

## Also found (not yet fixed, for whoever takes M4 next)

- Overlay backend `lisa-overlayd` exported-object mystery from the
  2026-07-23 session: after a gdm bounce one backend instance owned
  `org.lisa.Overlay1` but answered "Object does not exist" on Ask.
  Could not reproduce after restart; suspect activation race. Watch it.
- `org.gnome.Shell.Eval` is blocked on the image (returns `(false,'')`)
  — remote shell introspection needs busctl/gdbus only.
- Box suspends aggressively; SSH drops. Wake: physical nudge or re-ping.

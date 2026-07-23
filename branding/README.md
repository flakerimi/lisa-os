# Lisa OS — brand kit

The identity for Lisa OS: warm, personal, local. Not cold tech — a
computer that's genuinely yours to think with ("Her, but yours", see
`docs/VISION.md`).

## Assets

- `lisa-mark.svg` — the mark (512×512 squircle). Used as the OS logo:
  shipped as `lisa-logo` under `hicolor/scalable/apps`, referenced by
  `LOGO=lisa-logo` in os-release, so it shows in Settings › About.
- `lisa-wordmark.svg` — horizontal lockup (mark + "Lisa OS").

The mark is a warm presence — an orb, gently lit — held in an **indigo**
squircle: calm intelligence cradling a warm companion that lives on your
hardware, not in a cloud. Indigo is the UI/brand color; the warmth lives
in the orb and warm-tinted neutrals, *not* an orange accent. The
wordmark's text uses a system sans fallback; outline it to paths before
any print / external use so it renders identically everywhere.

## Palette (core)

| Token | Hex | Use |
|---|---|---|
| Indigo 500 | `#4F46E5` | PRIMARY brand/UI, the "OS" in the wordmark |
| Indigo 300 | `#6D63FF` | gradient top, hovers |
| Indigo 700 | `#4338CA` | pressed, gradient base |
| Warm Orb | `#FFD3B8`→`#FFF` | the logo orb (identity warmth only) |
| Ink | `#2B2320` | wordmark text, on-light foreground |
| Warm White | `#FFF1E9` | on-dark text, warm surfaces |

`ANSI_COLOR` in os-release is Indigo 300 (`1;38;2;109;99;255`) — the
brighter step reads better on a dark console. The full UI token system
(warm neutrals, dark surfaces, semantic colors) and the surfaces to design
live in [`DESIGN_BRIEF.md`](DESIGN_BRIEF.md) — a ready-to-paste prompt for
designing Lisa OS interfaces.

Orange is deliberately **out** of the brand: egress amber (`#E66100`) is
reserved and load-bearing — it means "leaves your hardware" (ADR-0008,
§5.11). An indigo brand keeps the warning unmistakable; nothing in the UI
competes with it.

## Where it's wired

- `os/mkosi/mkosi.postinst.chroot` rebrands os-release (`NAME`/
  `PRETTY_NAME` = "Lisa OS", `ID=lisa`, `LOGO=lisa-logo`, indigo
  `ANSI_COLOR`, our URLs).
- `os/mkosi/mkosi.extra/usr/share/icons/hicolor/scalable/apps/lisa-logo.svg`
  — the logo icon the About panel resolves.

This is a first identity pass. Next: GDM/greeter logo, a boot splash
(Plymouth), and a default wallpaper in the same language.

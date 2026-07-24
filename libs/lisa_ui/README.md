# lisa_ui — the widget kit Lisa apps import

Spec: docs/PLAN.md §5.12. Milestone: M6. Governance: ADR-0014 (phase 1),
ADR-0004 (history).

One import gives an app the whole vocabulary:

```dart
import 'package:lisa_ui/lisa_ui.dart';
```

- **Material-backed (phase 1):** a curated re-export of
  `package:flutter/material.dart` — app structure, navigation, buttons,
  inputs, lists, dialogs, feedback, theming. When Flutter's
  Material/Cupertino decoupling into packages lands, phase 2 swaps the
  backend to a vendored fork with no app-facing API change (ADR-0014).
- **Lisa theming:** `LisaApp` (a `MaterialApp` pre-wired to `lisaTheme`)
  derives light + dark schemes from the violet seed `Color(0xFF6D45C9)`,
  Material 3, Rubik as the default font. Rubik is not bundled and there is
  no google_fonts dependency — the family resolves against the
  OS-installed font and falls back to the platform default sans when
  absent.
- **Lisa widgets:** `LisaScaffold`, `LisaCard`, plus the AI-native
  `LisaStreamText` and `ConsentChip` that Material doesn't have.
- **Tokens:** `LisaTokens`/`LisaTheme` are unchanged; the theme file
  integration (Appendix E) will replace `LisaTokens.fallback` with live
  system values.

Status: **phase 1 landed** — Material-backed kit under widget tests.

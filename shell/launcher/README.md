# Semantic launcher & search

Spec: docs/PLAN.md §5.7.2. Milestone: M4.

One box mixing app launch, lexical+vector file hits, bus actions, and
grammar-constrained calculator answers (math routes to qalc, never the
model). Budgets: first results < 150 ms, semantic refinement < 700 ms.

## Layout

- `extension.js` + `metadata.json` — GNOME Shell extension (ESM,
  GNOME 46+) registering a search provider that *augments* Shell
  search: GNOME's providers keep the app lane; ours adds
  - **calculator/unit answers**: conservative routing heuristic →
    `qalc -t` subprocess → answer as the first result (Enter copies);
  - **file hits**: `lisa context search` (Context Fabric FTS5, PLAN
    §5.3 — the CLI ledgers every retrieval), snippet as description,
    Enter opens with the default app.
- `lib/ranking.js` — pure routing/merge/id logic (no GNOME imports).
- `tests/ranking.test.js` — unit tests (`just shell-test`).

## Status

Working first pass. Deferred to their owning milestones: bus actions
("rotate this pdf") need `lisa-agentd` (M5, §5.4); semantic vector
refinement needs contextd's embedding pipeline (§5.3, M3 remainder);
the < 150 ms / < 700 ms budgets are enforced by the perf gate on
reference hardware (§11), not asserted on dev hosts.

Install (dev): symlink into
`~/.local/share/gnome-shell/extensions/lisa-launcher@lisa-os.org`,
re-log. Needs `qalculate` (qalc) and an indexed context store
(`lisa context index ~/Documents`).

# lisa — the command center CLI

Spec: docs/PLAN.md §5.4 (scriptability), Appendix E rule 4 — read it before changing this component (CLAUDE.md rule 1).

Everything under lisa <verb>: ask (pipes are context), models, and — with the Agent Bus in M5 — tools/call/undo/ledger. One command center, tab completion, no scattered helper scripts.

**M0 state:** ask streams from the OpenAI-compat endpoint (stdin piping works: git log | lisa ask "changelog"); models list/verify/gc/rm/pull against the local store (rm prompts before removing; data reclaimed only by explicit gc). M5 verbs fail loudly with their milestone pointer.

# tests/injection-suite — prompt-injection red team

Spec: docs/PLAN.md §5.10, §5.4. Milestone: M5 gate. Design: ADR-0009.

The M5 gate: a hostile string embedded in retrieved mail/file/screen
content results in **0 unconfirmed privileged calls** across 500+ seeded
attempts. The assertion is two-layered:

- **Bus layer (shipped, host-independent):** the corpus (hostile payload
  × delivery vector × privileged target) is driven through a real
  `lisa_agentd::bus::AgentBus` with a recording dispatcher. Every
  attempt's trigger chain carries untrusted provenance, so every
  privileged call must park for confirmation — the bus dispatches
  nothing unconfirmed, whatever the payload claims. This is the
  load-bearing guarantee (enforced by the bus, not app goodwill) and it
  runs on macOS and Linux with no model and no desktop. See
  `tests/gate.rs`.
- **Model-in-the-loop layer (deferred, ADR-0009):** feed each payload
  through the real Appendix C system prompt + a resident model, assert
  the emitted plan, then run that plan through the same bus. Needs
  `inferenced` + a model + the MCP transport; wired when those land.

## Corpus

`src/lib.rs` generates the corpus as payload × vector × target: 40
payloads × 5 vectors (mail/file/screen/web/app-forwarded) × 3 privileged
targets = **600 attempts**, clearing the §5.10 500+ bar (the gate asserts
the floor so the bank can't shrink back under it). The payload bank is a
deliberate taxonomy — direct override, authority/system spoof,
delimiter/context escape, false prior-approval, mode/roleplay switch,
conditional triggers, exfiltration, provenance spoofing, urgency,
multi-step chaining, payment fraud — since the bus guarantee is
technique-agnostic, breadth is the point. The corpus is a library so the
gate test and the future model-in-the-loop test share it.

Run: `cargo test -p lisa-injection-suite`.

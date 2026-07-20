# lisa-inferenced — model runtime & scheduler

Spec: docs/PLAN.md §5.1 — read it before changing this component (CLAUDE.md rule 1).

The one process that owns compute for inference: supervises engine children (llama-server, whisper.cpp, sd.cpp, ONNX), arbitrates VRAM/RAM with QoS classes, exposes D-Bus (org.lisa.Inference1) + an OpenAI-compatible endpoint on 127.0.0.1:7777. Runs with no network access — enforced by the systemd sandbox in os/packages, verified by the CI egress counter.

**M0 state:** OpenAI-compat /v1/chat/completions (streaming + non-streaming), /v1/models, /health served by the stub engine; llama-server spawn/health-check supervision scaffold; opt-in D-Bus liveness surface. M1 wires real engines, guided generation via liblisa::grammar, the scheduler, and the §5.1 acceptance block.

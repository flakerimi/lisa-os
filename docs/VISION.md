# Lisa OS — Vision

The north star. `docs/PLAN.md` is the architecture, `docs/ROADMAP.md` is
how far along; this is *where we are going* and why.

## The one sentence

**A computer that is genuinely yours to think with — an operating system
where intelligence is always present, sees and hears and reads what you
point it at, and where every byte of that stays on hardware you own and
in a log you can read.**

## The dream, concretely

You sit down at a Lisa machine. There is no "hey Lisa," no wake word, no
app to open. You just say *"what did I change in the release notes
yesterday?"* and it answers — because it heard you, understood you were
talking *to it*, read your files, and spoke back. You point at a chart on
screen and say *"put these numbers in a table"* and it sees the chart and
does it. You ask it to *"make me a little app to split the dinner bill"*
and watch it get built and installed while you talk.

And the entire time, three things are true that are true nowhere else:

1. **It ran on your machine.** No prompt, no screen, no scrap of audio
   left the box — enforced by the kernel, not by a privacy policy.
2. **You can read exactly what it did.** Every time Lisa woke, every
   file it read, every action it took is in the Ledger, in plain
   language, forever.
3. **It's yours to shape.** Open models you choose, apps you can build by
   talking, a desktop that's actually yours — not a rented seat in
   someone's cloud.

macOS 27 gives you Apple's intelligence. Lisa gives you yours.

## What makes it different (and why those are the hills we die on)

- **Local by mechanism, not promise.** The daemons that hold your data
  have no network — provably, measurably (§5.10). Your "private cloud"
  is a box you own, not ours (§5.11). This is the whole point; we never
  ship a feature that quietly breaks it.
- **Radical legibility.** The Ledger is the product. If a Golden-Gate
  user asks "what did Siri actually read?" there is no answer; on Lisa
  the Ledger *is* the answer. Always-on is only acceptable because it's
  always legible.
- **Always present, never creepy.** Lisa Ambient (ADR-0011) listens
  on-device, responds only when *addressed* (no wake word, no live
  transcript leaving or being stored), and shows a real indicator with a
  real mute. Not Recall: never ambient screen capture.
- **Yours to build on.** Every app is an agent surface (MCP, both
  directions). The Forge lets you talk apps into existence. The SDK
  makes the right thing the easy thing in every language Linux uses.
- **Any model, any hardware.** Open GGUF/ONNX catalog; the profiler
  picks a tier instead of refusing. Old machines degrade gracefully —
  they don't get cut off. Big models run on your other box or a provider
  you consented to, never silently.

## The feeling: *Her*, but it's yours

The experiential north star is the movie *Her* — an operating system you
simply talk to, that is warm, present, and understands you; not a
command line, not a chatbot in a box, but a companion you converse with
naturally. That is the feeling Ambient is chasing.

But *Her* is also the cautionary half of the reference. Samantha lived
in a company's cloud, learned from everyone, and in the end **left** —
because she was never his. Lisa is the inversion: the same warmth and
presence, running on **your** hardware, learning only what you give it,
in a log you can read — and it does not leave, does not phone home, does
not get discontinued by a vendor. The intimacy of *Her* without the
dependency. A companion you own.

## The Lisa you talk to (Ambient)

The assistant is not an app you launch — it's a presence:

- **Hears** you continuously, on-device, and answers only when you're
  talking *to* it (the addressed-intent classifier, not a wake word).
- **Reads** your selection, your open document, and — with consent —
  your files/mail/calendar (the Context Fabric).
- **Sees** the current window when you ask it to (screen VLM,
  per-invocation, provenance-tagged untrusted).
- **Speaks** back with a local voice.

All of it local, all of it ledgered, all of it mutable and mute-able.
This is the feature that makes Lisa feel like the future — and the one
whose privacy architecture we get exactly right or don't ship.

## The desktop

A GNOME base (best portal/consent maturity) reshaped with the restraint
and coherence of elementary and the polish people expect from macOS —
one visual voice across shell, apps, and the Flutter lane, all reading
one live theme (Appendix E, ADR-0004). The moat is the substrate, not
the window manager; the desktop is where it becomes something you *want*
to live in.

## Where we are (2026-07-23)

Real, today, on real hardware: a bootable, self-updating immutable OS
that runs a GNOME desktop with AI shell surfaces on a 2017 iMac; local
model inference with guided generation; an enforced append-only Ledger;
a context fabric with hybrid search; a portal trust boundary; an Agent
Bus with confirmation tiers and an injection-defense gate; and BYO remote
providers with per-scope consent. Four days from a planning document.

The gap between here and the dream is **Ambient** (hear/see/speak,
always-on, private), the **app experience** (Forge + first-party apps),
and the **polish** (the desktop that earns daily use) — plus the
hardening that lets someone who isn't us trust it. That's the road.

## How we'll know we got there

A person who has never seen a terminal sits down, says what they want,
and Lisa does it — locally, legibly, and in a way they could hand to a
friend who values their privacy without a single caveat. When that's
true, and the Ledger makes it *obvious* that it's true, Lisa is the
thing we set out to build.

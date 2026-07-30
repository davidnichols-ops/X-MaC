# X-MaC v1 SCOPE FREEZE

**Status:** ACTIVE
**Effective:** 2026-07-30
**Triggered by:** MAOS task #[SCOPE-FREEZE]

## What this means

No new engine, subsystem, intelligence layer, or abstract interface may be
added to X-MaC until this freeze is lifted. Bug fixes, safety hardening,
performance work, and docs for existing code are allowed. Scope-reducing
work (deleting dead code, removing features) is required, not optional.

## The 3 v1 capabilities (drilled to single user actions)

1. **Find safely-deletable duplicate files.**
   One CLI/GUI action: pick a directory, get a reviewable list of duplicate
   groups with a recommended canonical file, preview the reclaim, approve,
   verify. Powered by `engines/duplicate` + `engines/disk`.

2. **Explain why my disk is full.**
   One CLI/GUI action: scan the filesystem, get a category breakdown
   ("caches 32%, dev artifacts 18%, media 24%, unknown 26%") with the
   top reclaimable items per category and the evidence behind each
   recommendation. Powered by `twin/` + `engines/disk`.

3. **System health scan + recommendations.**
   One CLI/GUI action: scan the Mac (disk, apps, privacy, hardware), get a
   prioritized list of recommendations with confidence and risk, preview
   each, approve, verify. Powered by `engines/envmap` + `engines/privacy`.

## Why these three form a coherent product

- All three start from a **filesystem scan** — share the discovery layer.
- All three end in a **reviewable, reversible recommendation** — share the
  safety model (preview / approve / verify / rollback).
- All three answer one user question: **"what can I safely do with this
  Mac right now?"** (1 = delete dupes, 2 = understand my disk,
  3 = fix what's wrong).
- They are not three unrelated surfaces. Removing any one makes the other
  two weaker.

## What is NOT v1 (deferred to v2 or archive)

- GNN scoring of recommendations (move from "always on" to "opt-in
  experiment behind a feature flag"). See X-MAC-TWIN tasks.
- Privacy engine recommendations (data stays collected; recommendations
  stay disabled until safety review).
- Hardware reasoning / system tune suggestions (twin/reasoning.rs).
- Software genome construction (twin/software_genome.rs).
- Per-file perceptual similarity beyond exact duplicates.
- Any new dashboard / chart / category beyond what the 3 actions need.

## Definition of Done (v1)

A v1 capability is done when ALL of the following are true for the
single user action:

- [ ] Installable via documented path (Homebrew tap or DMG).
- [ ] One CLI subcommand runs it end-to-end.
- [ ] Produces a measurable result (reclaimed bytes, items scanned,
      time, recommendations).
- [ ] Every destructive recommendation shows a trusted preview.
- [ ] Every destructive action is reversible via snapshot rollback.
- [ ] Benchmarked on a real 500K+ file corpus with published numbers.
- [ ] macOS app is signed + notarized.
- [ ] User-facing README section exists for the capability.

## Dead-code removal (Task #128)

274 `#[allow(dead_code)]` annotations exist across 10 modules. The freeze
requires a survey of which ones correspond to v2/experimental/archive code
vs. genuine mistakes. Survey tracked under MAOS task #[SCOPE-DEADCODE].
Actual removal happens after the 3-capability decision is locked.

## Lifting the freeze

The freeze is lifted only when:

1. All three capabilities have a working `xmac <capability>` CLI command.
2. The product truth table (Task #130) is reviewed and committed.
3. The Definition of Done is reviewed and committed.

Until then, new work is restricted to: bugs, safety, perf, docs, dead-code
removal.
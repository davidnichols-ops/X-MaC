# X-MaC Dead-Code Survey

**Date:** 2026-07-30
**Triggered by:** MAOS task #527, SCOPE freeze #123
**Result of:** `grep -rE '#\[allow\(dead_code\)\]' --include='*.rs' src/`

## Summary

- **Total `#[allow(dead_code)]` annotations:** 274
- **Files affected:** 19+ (top 10 below; full list in `dead_code_inventory.txt`)
- **Bulk of dead code lives in:** `src/twin/` (digital twin), `src/engines/duplicate/`,
  `src/engines/disk/`, `src/engines/envmap/`, `src/intelligence/`

## Top files by count

| File | Allows | Role | Classification |
|---|---|---|---|
| `src/twin/hardware.rs` | 39 | Hardware reasoning | **v2 / archive** — not in 3 v1 capabilities |
| `src/engines/duplicate/engine.rs` | 32 | Dedup engine | **v1 keep** — capability #1; allows are future-use fields (`scan_paths`, `similar_images`, `with_config`) that should be removed as we tighten the API |
| `src/twin/reasoning.rs` | 27 | GNN / rule reasoning | **v2 / archive** — GNN auto-scoring is deferred per SCOPE_FREEZE.md |
| `src/engines/disk/engine.rs` | 21 | Disk categorization | **v1 keep** — capabilities #2 & #3 depend on it |
| `src/engines/envmap/discovery.rs` | 16 | System env discovery | **v1 keep** — capability #3 depends on it |
| `src/twin/fs_graph.rs` | 14 | Filesystem graph builder | **v1 keep (capability #2)** — but needs review: only the graph nodes/edges needed for "explain why disk is full" should stay; GNN-specific extras should be archived |
| `src/twin/software_genome.rs` | 13 | Software inventory | **archive** — not in 3 v1 capabilities |
| `src/engines/envmap/apps.rs` | 11 | App inventory | **v1 keep** — capability #3 (system health scan) |
| `src/engines/privacy/engine.rs` | 9 | Privacy engine | **v1 data-collection only** — recommendations deferred per SCOPE_FREEZE.md |
| `src/cleanup/history.rs` | 7 | Cleanup history | **v1 keep** — needed for the "verify" step in the safety state machine |

## Classification rules

- **v1 keep:** code path is used by one of the 3 v1 capabilities (`xmac dedup`,
  `xmac disk --explain`, `xmac scan --recommend`).
- **v2 keep:** not used by v1 capabilities, but is a planned v2 capability
  (e.g. pHash perceptual similarity, GNN auto-scoring when behind a flag).
- **archive:** no plan to use; remove the file or move it behind a non-default
  feature flag.
- **mistake:** genuinely unused; remove the annotation and the code.

## Per-module classification (initial pass — needs review before any delete)

### v1 keep (do not delete)
- `src/engines/duplicate/engine.rs` — clean up `with_config` / `similar_images` /
  future fields that the new minimal CLI doesn't need. Target: reduce 32 → ~10.
- `src/engines/disk/engine.rs` — keep; review for category-only fields.
- `src/engines/envmap/discovery.rs`, `apps.rs` — keep.
- `src/twin/fs_graph.rs` — keep only the parts needed by capability #2
  ("explain why my disk is full"); split off GNN-specific node features into
  `gnn/` or archive.
- `src/cleanup/history.rs` — keep.

### v2 / archive (gate behind feature flag or delete)
- `src/twin/hardware.rs` — capability not in v1; move to `src/engines/hardware/`
  behind `--experimental` or delete. **39 allows** → high payoff.
- `src/twin/reasoning.rs` — GNN inference; gate behind `--gnn` flag in v2.
  **27 allows** → high payoff.
- `src/twin/software_genome.rs` — not in v1; archive. **13 allows**.
- `src/twin/knowledge_graph.rs`, `src/twin/process.rs`, `src/twin/memory.rs`,
  `src/twin/energy.rs`, `src/twin/event_stream.rs`, `src/twin/app_agent.rs` —
  audit individually; most should archive.
- `src/engines/optimize/`, `src/engines/diag/`, `src/engines/conflict/`,
  `src/engines/maintain/`, `src/engines/depth/`, `src/engines/graph/`,
  `src/engines/clean/`, `src/engines/map/`, `src/engines/startup/`,
  `src/engines/privacy/` — most have large `dead_code` counts because they
  expose builder/config methods for an older API surface. Map each to one
  of the 3 v1 capabilities, or move to v2/archive.

### Mistake (remove annotation AND code)
- `src/util/progress.rs` — `ProgressReporter` struct + impl marked
  `#[allow(dead_code)]`. Either wire it into the output path
  (also addresses task #150, "no fake percentages") or remove.
  If removed: removes 2 allows and one unused file.
- `src/util/macos.rs` — `MacosUtils::new()` constructor is dead; either use
  it in main or delete.
- `src/intelligence/advisor.rs` — sample of allows look like genuinely
  unreferenced helper methods; review and remove.

## Recommended execution order

1. **Decide v1 keep vs archive per file** (this survey is the input — needs
   human review to confirm).
2. **Move `src/twin/hardware.rs` + `reasoning.rs` behind `--experimental`** —
   smallest change, biggest payoff (66 allows gated).
3. **Delete `src/twin/software_genome.rs`** — clearly out of scope for v1.
4. **Tighten `src/engines/duplicate/engine.rs`** — remove `with_config` if
   CLI doesn't use it, remove `similar_images` field if not wired to CLI.
5. **Wire or remove `ProgressReporter`** (kills two tasks at once: this
   survey item AND task #150).
6. **Audit `engines/*` builders** — most have dead `with_*` methods for an
   older configuration API; replace with the minimal struct literal that
   the CLI actually uses.

## Validation

After each delete pass:
- `cargo build --release --bin x-mac` must still succeed.
- `cargo test --release --lib` must still pass with the same number of tests.
- `xmac dedup /tmp/xmac-dedup-test` must still return the same finding
  (this corpus is the regression fixture).

## Open question for human review

Before any deletes:
- Is `software_genome.rs` data being consumed by any external integration
  (MCP, JSON export)? If yes, gate behind a flag instead of deleting.
- Does any task in MAOS reference `engines/hardware` or `engines/conflict`
  as a v1 dependency? (Quick `grep` suggests no — they are task subjects
  but not consumers.)

Final inventory is captured in `dead_code_inventory.txt` (full grep output).
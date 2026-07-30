# X-MaC v1 Product Truth Table

**Date:** 2026-07-30
**Triggered by:** MAOS task #526, SCOPE freeze #123

Per SCOPE_FREEZE.md, v1 has exactly three user-visible capabilities, each
drilled to a single user action. This table is the contract: every capability
must answer all six columns before it ships.

---

## Capability 1: Find safely-deletable duplicate files

| Column | Value |
|---|---|
| **User action** | `xmac dedup ~/Downloads --min-size 1M` (one CLI subcommand; GUI button is the same surface) |
| **Input** | A path (file or directory) plus optional `--min-size`, `--similar` (perceptual), `--exclude` patterns |
| **Computation** | Walkdir traversal → size-group → partial BLAKE3 (first+last 4KB) → full BLAKE3 → cluster by hash → rank canonical file (newest + path priority) |
| **Output** | One `Finding` per duplicate cluster: `keep_path`, `delete_paths`, BLAKE3 hash, confidence (0..1), reclaimable bytes. Always reviewable; nothing deleted unless user invokes `xmac purge` |
| **User benefit** | User sees exactly which files are duplicates, which one to keep, and how much space can be reclaimed — without deleting anything by default |
| **Measurable outcome** | Cluster count, reclaimable bytes, scan duration, false-positive rate (manual review against known-clean corpus) |
| **Failure mode** | Hash collision (BLAKE3 256-bit: practically impossible for user files), false positive on perceptual mode (pHash collision), partial-hash false positive on large files (mitigated by full-hash phase) |

**Evidence it works today (2026-07-30):**
- `xmac dedup /tmp/xmac-dedup-test --min-size 1K` returns 1 cluster of 3 identical files, correctly identifies `original.dat` as keep and 2 copies as delete candidates. BLAKE3 hash match, 100% confidence.
- `cargo test --release --lib engines::duplicate` → 73 passed, 0 failed.
- Built with `cargo build --release --bin x-mac` (9m 09s, no errors).

**What is NOT in scope for capability 1 (per SCOPE freeze):**
- `--similar` (perceptual image hashing) is implemented but not promoted; documented in `engines/duplicate/scanner.rs` pHash DCT implementation; v2 feature.

---

## Capability 2: Explain why my disk is full

| Column | Value |
|---|---|
| **User action** | `xmac disk ~/Projects --explain` (one CLI subcommand) |
| **Input** | A path, `--top N` (default 30), `--by category` (default), `--category <name>` to filter |
| **Computation** | Walkdir traversal with size aggregation, twin graph construction (nodes = dirs/files, edges = parent/child + symlink), category classifier (caches / dev artifacts / media / archives / applications / backups / duplicates / unknown) |
| **Output** | Human-readable breakdown: total size, category breakdown with percentages, top N reclaimable items with evidence (path, size, mtime, why-this-category), JSON for scripting |
| **User benefit** | User understands their disk — not just "Downloads is 3.1 GB" but "caches account for 32%, dev artifacts 18%, media 24%; these three categories are 83% of reclaimable space" |
| **Measurable outcome** | Scan time on real machines, breakdown accuracy (vs. manual classification), reclaimable estimate vs. actually-reclaimed-after-clean |
| **Failure mode** | Slow walkdir on huge trees (mitigated by `--exclude`, resource modes), category mis-classification (mitigated by conservative defaults + always-show-evidence), symlink loops (mitigated by default `--follow-symlinks=false`) |

**Evidence it works today (2026-07-30):**
- `xmac disk` subcommand exists; evidence pending until scope completes (--explain flag may or may not exist; check during implementation).
- `engines/disk/engine.rs` has 21 dead-code allows — review pending per dead-code survey.

**What is NOT in scope for capability 2:**
- GNN scoring of recommendations (v2, behind `--experimental`).
- Per-file "should I delete this" advice (that's capability 3).

---

## Capability 3: System health scan + recommendations

| Column | Value |
|---|---|
| **User action** | `xmac scan --recommend` (or `xmac doctor --recommend`) |
| **Input** | The full system (privacy-redacted by default), `--include <engine>` to limit, `--severity <level>` filter |
| **Computation** | Run envmap (OS, packages, apps), disk categorization, startup items, privacy posture; aggregate into prioritized recommendation list; classify each as safe / review / protected |
| **Output** | Ranked list of recommendations with: title, what-it-is, why-it-matters, evidence (path / process / setting), safety rating, proposed action, undo metadata |
| **User benefit** | User gets a prioritized "what to fix on this Mac" list with the evidence and the undo path — no AI confidence overrides safety |
| **Measurable outcome** | Recommendation count, false-positive rate (manual review), time-to-first-fix, recommendations-acted-on rate |
| **Failure mode** | Privacy leak (mitigated by `engines/envmap/redaction.rs`), false positive on recommendation (mitigated by always-evidence + safety rating), destructive action taken without consent (mitigated by read-only default; only `xmac purge` modifies filesystem) |

**Evidence it works today (2026-07-30):**
- `xmac scan`, `xmac doctor`, `xmac envmap`, `xmac startup`, `xmac privacy` all exist as subcommands.
- `engines/envmap/{discovery,apps}.rs` and `engines/privacy/engine.rs` exist.
- Privacy recommendations explicitly deferred per SCOPE_FREEZE.md — capability 3 ships with envmap + startup + disk categorization recommendations only.

**What is NOT in scope for capability 3:**
- Privacy engine recommendations (data stays collected; recommendations disabled).
- Hardware tuning / system modification suggestions (capability deferred).

---

## Cross-capability guarantees

All three capabilities share:

1. **Read-only by default.** No filesystem modification without explicit
   `xmac purge <plan>` invocation.
2. **Always show evidence.** Every recommendation cites the underlying file,
   path, process, or setting.
3. **Always reversible.** Every destructive action goes through the safety
   state machine: PREVIEW → APPROVED → EXECUTING → VERIFIED → ROLLBACK
   AVAILABLE.
4. **Never recommend on AI confidence alone.** A high-confidence model
   recommendation still passes the same action policy.

## Measurement plan

Before each capability is marked done (per Definition of Done), the following
must be measured and recorded in this file or a sibling benchmark doc:

- Time to scan a 500K-file corpus (cold + warm).
- Memory peak during scan.
- False-positive rate from manual review of 100 random findings.
- Time from scan complete to first user action.
- (Capability 1 only) Reclaimable bytes estimate vs. actual bytes reclaimed
  after `xmac purge`.

## Failure of this contract

If any capability cannot fill all seven columns with real evidence, that
capability is NOT v1. Move it to v2 or delete it.
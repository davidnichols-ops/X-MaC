# Operations Manifest Audit

**Generated:** 2026-07-30 at commit `54ee491`
**Source:** `docs/OPERATIONS_MANIFEST.md`
**Audit method:** For each row, parse target path → resolve to actual
file (with `src/` prefix) → search for op-name keywords (case-insensitive
substring match) in the target file.

This is a **first-pass audit**, not a semantic verification. It catches
claims that point to files where the cited operation is clearly absent
(text-level), not claims where the code uses different terminology.

## Summary

| Status marker | Count | Reachable target | Keyword present | Suspect |
|---------------|-------|------------------|-----------------|---------|
| `[E]` existing | 115 | 115 (100%) | n/a — pre-existing | 0 |
| `[X]` extend   | 83  | 83 (100%) | 73 (88%) | 10 |
| `[N]` new      | 422 | 420 (99.5%) | 401 (95.5%) | 19 |

- **2 ops** have unreachable targets: op 16 (`gui/XMacApp/DiskView.swift`,
  treemap view for Space Lens, marked "Roadmap v2.2") and op 69
  (`cleanup/executor.rs` — only `executor_tests.rs` exists).
- **29 ops** have keyword absence — the cited operation cannot be found
  by simple text search in the cited file.
- **Manifest drift:** the doc claims 630 ops but contains 620 rows; op IDs
  span 1–320, so some numbering is non-contiguous.

## Suspect claims requiring verification

These ops have `[X]` or `[N]` status but the cited file does not
contain the operation's keywords. They are candidates for either
removal from the manifest or actual implementation.

### `[X]` (extend) — extension never happened

| Op | Name | Target | Likely reality |
|----|------|--------|----------------|
| 7 | Track file creation dates | `src/engines/clean/scanner.rs` | Missing `ctime`/`creation` handling |
| 9 | Track access dates | `src/engines/clean/scanner.rs` | Missing `atime` tracking |
| 19 | Rank oldest files | `src/engines/clean/scanner.rs` | No age-based ranking helper |
| 20 | Rank unused files | `src/engines/clean/scanner.rs` | No usage-based ranking |
| 50 | Detect cache age | `src/engines/clean/scanner.rs` | No age detection in clean |
| 57 | Remove stale cache databases | `src/engines/clean/rules.rs` | No "stale DB" category |
| 59 | Analyze cache ownership | `src/engines/clean/scanner.rs` | No ownership metadata |
| 63 | Remove crash reports | `src/engines/clean/rules.rs` | No crash report rule |
| 64 | Remove diagnostic logs | `src/engines/clean/rules.rs` | No diagnostic log rule |
| 66 | Remove update logs | `src/engines/clean/rules.rs` | No update log rule |
| 68 | Detect oversized logs | `src/engines/clean/scanner.rs` | No size-based log detection |
| 159 | Generate performance reports | `src/cli/output.rs` | No perf report format |
| 167 | Monitor disk activity | `src/engines/optimize/telemetry.rs` | No disk activity tracking |
| 175 | Generate performance reports | `src/cli/output.rs` | Duplicate of op 159 |
| 308 | Expose APIs | `src/cli/output.rs` | No API exposure |

### `[N]` (new) — new feature never implemented

| Op | Name | Target | Likely reality |
|----|------|--------|----------------|
| 24 | Scan cloud-synced folders | `src/engines/clean/scanner.rs` | No iCloud/Drive/Dropbox detection |
| 25 | Detect iCloud Drive data | `src/engines/clean/scanner.rs` | |
| 26 | Detect Dropbox data | `src/engines/clean/scanner.rs` | |
| 27 | Detect Google Drive data | `src/engines/clean/scanner.rs` | |
| 28 | Detect OneDrive data | `src/engines/clean/scanner.rs` | |
| 55 | Detect corrupted caches | `src/engines/clean/scanner.rs` | No corruption check |
| 71 | Determine log importance | `src/engines/clean/rules.rs` | No log importance scoring |
| 70 | Track update history | `src/twin/software_genome.rs` | No history field |
| 72 | Track removal dates | `src/twin/software_genome.rs` | No removal field |
| 96 | Map backups | `src/twin/fs_graph.rs` | No backup node type |
| 97 | Map cloud files | `src/twin/fs_graph.rs` | No cloud-synced node type |
| 256 | Analyze slow boots | `src/engines/startup/engine.rs` | No boot time analysis |
| 257 | Profile application startup | `src/engines/optimize/engine.rs` | No startup profiling |
| 300 | Act as a macOS "systems operator" | `src/twin/reasoning.rs` | Aspirational |

## Confirmed-verified ops (spot checks)

| Op | Name | Evidence |
|----|------|----------|
| B.1 | Identify exact Mac model | `detect_mac_model()` at `src/twin/hardware.rs:403` |
| B.2 | Identify SoC generation | `detect_soc_generation()` at `src/twin/hardware.rs:418` |
| B.247 | Intelligent uninstall | `intelligent_uninstall()` at `src/cleanup/transaction.rs:410` |

## Recommended actions

### Immediate (Tier 2.4 of DoD)

1. **Update the manifest** — change `[X]`/`[N]` to `[X-pending]`/`[N-pending]`
   for the 29 suspect rows, so they don't masquerade as shipped work.
2. **Fix the 2 unreachable targets**:
   - Op 16: change target to `gui/XMacApp/` (no DiskView.swift exists) or
     mark as `[N-roadmap]` for v2.2.
   - Op 69: change target to `src/cleanup/transaction.rs` (where
     intelligent_uninstall actually lives).

### Strategic

3. **Decide each suspect row's fate**: implement, defer, or remove.
   The clean engine is the largest source of stale claims — many ops
   there read like a wishlist that became the doc.
4. **Add a CI check** that fails if a `[E]`/`[X]` row's target path
   doesn't resolve, or if the cited keyword can't be found.

## How to reproduce this audit

```bash
python3 -c "
import re, os, subprocess
# ... (full script in scripts/audit_manifest.py, to be added)
"
```

A reusable script will be added at `scripts/audit_manifest.py` so this
audit can run in CI.
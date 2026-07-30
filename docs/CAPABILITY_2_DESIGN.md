# X-MaC Capability #2 — Explain Why My Disk Is Full

## What the user types

```bash
xmac disk ~/Projects --explain
xmac disk / --top 50 --by category
```

## What they get

A category breakdown showing:

1. **Volume summary** (total / used / free) — already exists
2. **Category breakdown** with percentages, sorted by reclaimable size:
   - `caches` — derived from `Cache`, `BrowserCache`, `PackageManagerCache`, `Log`
   - `dev artifacts` — derived from `BuildArtifact`, `XcodeArtifact`, `PackageManagerCache` (npm/cargo), `PythonEnv`, `NodeEnv`
   - `media` — images, video, audio by extension
   - `archives` — `.zip`, `.tar*`, `.dmg`, `.pkg`, `.7z`, `.rar`
   - `applications` — `.app` bundles, InstalledApp
   - `backups` — `.iosbackup`, `MobileSync`, Time Machine paths
   - `duplicates` — DuplicateFile (cross-link to capability #1)
   - `unknown` — everything else

3. **Top reclaimable items per category** with the evidence:
   - Path
   - Size
   - Modification time
   - Why-this-category (the matching path/extension rule)

## Implementation strategy

Rather than building a new scanner, **classify the existing scan findings** into the 8 buckets using a path-based heuristic. This is fast (no extra disk I/O) and reuses the existing walk infrastructure.

### Classifier (`src/engines/disk/classifier.rs`)

```rust
pub enum Bucket {
    Caches,
    DevArtifacts,
    Media,
    Archives,
    Applications,
    Backups,
    Duplicates,
    Unknown,
}

pub fn classify(path: &Path) -> Bucket;
```

Rules:
- Path-based first (more reliable than extension):
  - `/Library/Caches/`, `~/.cache/`, `~/Library/Caches/` → Caches
  - `~/Library/Logs/`, `*.log` → Caches (logs are cache-like)
  - `node_modules/`, `target/`, `.gradle/`, `build/`, `dist/` → DevArtifacts
  - `DerivedData/`, `*.xcuserstate` → DevArtifacts
  - `.Trash/` → Backups
  - `MobileSync/`, `*.iosbackup`, `Backups.backupdb/` → Backups
  - `*.app/`, `/Applications/` → Applications
  - `*.dmg`, `*.pkg`, `*.zip`, `*.tar*`, `*.7z`, `*.rar` → Archives
- Extension-based fallback:
  - Images/video/audio → Media
- Default: Unknown

### Aggregation pass

Walk all `SystemInfo` + `LargeFile` findings emitted by the existing scan,
classify each into a bucket, accumulate per-bucket:
- total_bytes
- count
- top-N largest paths (by size)

### Output format (`--explain`)

JSON (always) + human-readable summary (in `--format report`).

JSON shape:
```json
{
  "volume": { "total": ..., "used": ..., "free": ... },
  "categories": [
    {
      "bucket": "caches",
      "total_bytes": 12345678,
      "percent_of_used": 12.5,
      "item_count": 42,
      "top_items": [
        { "path": "...", "size": ..., "modified": ..., "reason": "..." }
      ]
    }
  ],
  "items_scanned": 523412,
  "duration_secs": 4.2
}
```

### What this is NOT

- It does not run an extra scan. It classifies the existing scan output.
- It does not delete anything. `--explain` is read-only.
- It does not use the GNN. Per SCOPE freeze, GNN auto-scoring is v2.
- It does not call out specific apps or services. Conservative path-based rules.

## Files to add/modify

1. `src/engines/disk/classifier.rs` — new file with `Bucket` enum + `classify()` function + tests
2. `src/engines/disk/mod.rs` — add `pub mod classifier;`
3. `src/engines/disk/engine.rs` — add `--explain` and `--by category` flags; emit an extra aggregate finding when `--explain` is set
4. `src/cli/args.rs` — add `explain: bool` and `by_category: bool` to `DiskArgs`

## Tests

- `classifier::tests::test_classify_caches` — `~/Library/Caches/com.apple.Safari`
- `classifier::tests::test_classify_dev_artifacts` — `node_modules/foo/index.js`
- `classifier::tests::test_classify_media` — `Vacation.jpg`
- `classifier::tests::test_classify_archives` — `archive.zip`
- `classifier::tests::test_classify_applications` — `Some.app/Contents/Info.plist`
- `classifier::tests::test_classify_backups` — `Backup.iosbackup`
- `classifier::tests::test_classify_unknown` — `random_file.xyz`

## Evidence requirements

Per Definition of Done for capability #2:
- [ ] Scan completes on a 500K+ file real corpus without crashing
- [ ] Cold scan time measured and reported
- [ ] Output JSON validates against the shape above
- [ ] Every category shows non-zero `top_items` on the test corpus (or the bucket is absent)
- [ ] No findings claim reclaimable bytes for system files (conservative classification)
- [ ] Manual review: spot-check 10 random top items against Finder to confirm the category label is right

## Out of scope (deferred)

- Confidence scores per recommendation (v2 — capability #3 will own this)
- "Safe to delete" verdicts (capability #3 owns the safety policy)
- Category-specific recommendations (which cache to clean) — that's `xmac clean`
- GNN scoring
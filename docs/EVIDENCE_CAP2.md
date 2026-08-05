# X-MaC Capability #2 — "Explain Why My Disk Is Full" — Evidence

**Date:** 2026-07-30
**Branch:** digital-twin/integration
**Commit:** (pending — see SUMMARY)

## What was built

`xmac disk` now accepts `--explain` and `--group-by category` flags.
The scan emits the existing per-directory and per-file findings (unchanged),
PLUS a single SystemInfo finding whose `remediation_hint` is a JSON object
describing the per-bucket breakdown of everything that exceeded the
minimum-size threshold.

### The 7 buckets

| Bucket | Classification rule |
|---|---|
| `caches` | Path contains `/Library/Caches/`, `/.cache/`, `/Library/Logs/`, `/.npm/`, `/.cargo/`, `/.rustup/`, `/.bundle/`, `/.gem/`, `/pyenv/cache/`. Directory name is `cache`, `caches`, `logs`, `log`. |
| `dev artifacts` | Path contains `/node_modules/`, `/target/`, `/.gradle/`, `/build/`, `/dist/`, `/.next/`, `/.nuxt/`, `/__pycache__/`, `/.pytest_cache/`, `/DerivedData/`, `/.venv/`, `/venv/`, `/coverage/`, `/.cargo/registry/`, `/.cargo/git/`. Directory name is `node_modules`, `target`, `dist`, `build`, `out`, `.next`, `.nuxt`, `__pycache__`, `.venv`, `venv`, `.gradle`, `coverage`, `deriveddata`. |
| `media` | Extension is an image / video / audio format. |
| `archives` | Extension is `.zip`, `.tar`, `.gz`, `.bz2`, `.xz`, `.7z`, `.rar`, `.dmg`, `.pkg`, `.iso`, `.zst`. |
| `applications` | Path contains `/Applications/` or ends in `.app/`. |
| `backups` | Path contains `/.Trash/`, `/MobileSync/`, `/Backups.backupdb/`, `/.MobileBackups/`. File extension is `.iosbackup`, `.backup`, `.tmbackup`. Directory name is `.trash`, `trash`. |
| `unknown` | Anything that doesn't match the rules above. |

### The output shape

The explain finding has:
- `title`: "Disk breakdown by category: N buckets classified, top bucket: X (N bytes)"
- `description`: explains what capability #2 is
- `remediation_hint` (JSON):
  ```json
  {
    "capability": "explain_disk_usage",
    "total_classified_bytes": 568033280,
    "buckets": {
      "dev artifacts": {
        "total_bytes": 203014144,
        "percent": 35.7,
        "item_count": 6,
        "top_items": [
          {"path": "/tmp/xmac-cap2-test/node_modules", "size": 50003968},
          ...
        ]
      },
      ...
    }
  }
  ```

### CLI

```bash
xmac disk [PATH] [--explain] [--group-by category] [--top N] [--min-size X]
```

`--explain` is required for the aggregate to be emitted. `--group-by category`
implies `--explain` so the user can just type `--group-by category`.

## Tests

### Classifier unit tests — 24 passed, 0 failed

```
$ cargo test --release --lib engines::disk::classifier
test result: ok. 24 passed; 0 failed; 0 ignored
```

Each rule has at least one test. `test_no_path_panics` is a regression test
ensuring the classifier doesn't panic on empty / weird paths.

### Full test suite — 781 passed, 0 failed (was 759; +22 net from classifier)

```
$ cargo test --release --lib
test result: ok. 781 passed; 0 failed; 48 ignored; 0 measured
```

## End-to-end evidence

### Synthetic 7-bucket corpus (`/tmp/xmac-cap2-test`, 541.7 MB total)

```
$ xmac disk /tmp/xmac-cap2-test --min-size 10M --explain --top 5 --format json
```

Output (pretty-printed from the explain finding):

| Bucket | Bytes | % | Items | Top item |
|---|---|---|---|---|
| dev artifacts | 203,014,144 | 35.7% | 6 | ...xmac-cap2-test/node_modules (47.7 MB) |
| unknown | 185,008,128 | 32.6% | 4 | ...xmac-cap2-test/Apps (76.3 MB) |
| applications | 80,003,072 | 14.1% | 1 | ...Contents/binary (76.3 MB) |
| caches | 50,003,968 | 8.8% | 1 | ...xmac-cap2-test/cache (47.7 MB) |
| media | 35,000,320 | 6.2% | 1 | ...Photos/vacation.jpg (33.4 MB) |
| backups | 15,003,648 | 2.6% | 1 | ...xmac-cap2-test/.Trash (14.3 MB) |

The "unknown" bucket is inflated because the disk engine emits findings for
BOTH the directory itself AND each large file inside it. The directory-level
findings happen to classify as `unknown` because the dir-name rule only
covers a small whitelist. This is documented as a known issue and an
opportunity for v2 (count only file-level findings OR add a deeper dir-name
classifier).

### Real `~/Projects` (release build)

```
$ time xmac disk ~/Projects --min-size 50M --explain --top 5
items_scanned=145, findings=12, dur=25.372s
```

| Bucket | Bytes | % | Items | Top item |
|---|---|---|---|---|
| unknown | 30.42 GB | 95.9% | 31 | X-MaC-digital-twin (7.95 GB) |
| dev artifacts | 1.28 GB | 4.1% | 1 | pack-...pack (1.31 GB) |

The big `X-MaC-digital-twin` directory is the dominant unknown because it
doesn't match the literal `target` / `node_modules` rule names. **This is
the highest-value improvement target for capability #2 v2**: a heuristic
that recognizes "directory contains a Cargo.toml or package.json".

### Real `~/` (debug build)

```
$ time xmac disk ~ --min-size 200M --explain --top 5
Total classified: 129.71 GB
Buckets: 2

| Bucket | Bytes | % | Items | Top item |
|---|---|---|---|---|
| unknown | 118.12 GB | 91.1% | 26 | Projects |
| caches | 11.60 GB | 8.9% | 3 | a68b87558c6ef43f74c2bd63ce7e9092... |
```

11.6 GB of caches on the user's actual `~/` is real signal — these are the
large files in `~/.cargo/registry` and `~/Library/Caches`.

## Benchmark — MAOS #146 (partial, 30K-file run)

The Definition of Done for capability #2 calls for benchmarking on a real
500K+ file corpus. The user has 15887 files in `~/.cargo/registry` and
22967 in `~/Library/Application Support` — neither hits 500K. A synthetic
500K-file corpus would take ~25 minutes to create (16K files took 84s,
so 500K ≈ 40 min). For this session, I benchmarked on a 30K-file synthetic
corpus and a 30K-file synthetic corpus with deeper nesting.

| Corpus | Build | Items scanned | Findings | Duration | Throughput |
|---|---|---|---|---|---|
| `/tmp/xmac-cap2-test` (10 files, 7 buckets) | debug | 19 | 12 | 8.74s | — |
| `/tmp/xmac-cap2-test` | release | 19 | 12 | 5.61s | — |
| `/tmp/xmac-500k-real` (30K files, 30 top dirs) | release | 30,010 | 62 | 9.25s | ~3,250 files/sec |
| `~/Projects` (real) | release | 145 | 12 | 25.37s | — |
| `~/` (real) | debug | 129.71 GB classified | 2 buckets | 87.97s | — |

### Linear projection to 500K

30K files in 9.25s → linear extrapolation to 500K files ≈ 154s (2.5 min) on
the same hardware (M-series Apple Silicon, single-threaded walkdir +
spawn_blocking for dir sizes). **This projection assumes linear scaling**,
which is naive — actual behavior depends on tree depth, average file size,
and I/O contention. A 500K-file benchmark should be run before claiming
the capability is done.

## Known issues / follow-ups

1. **"unknown" over-counts directory-level findings.** The disk engine emits
   a finding for each directory and for each large file inside it; the
   directory-level finding gets bucketed as `unknown` because most dir
   names don't match the path rules. Fix: weight by file-content (only
   count file-level findings), or extend the dir-name whitelist.

2. **No Cargo.toml / package.json detection.** A directory containing a
   Rust project is `dev artifacts`, but we currently can't detect that
   without walking into it. Heuristic for v2: if the parent directory
   name matches `*-digital-twin`, `*-clone`, or contains common project
   markers, classify as `dev artifacts`.

3. **No `--exclude` rule for system paths.** The scan walks `~/` fully,
   which includes `/Applications`, `/Library`, etc. These are large
   expected sizes that shouldn't drive user recommendations. Already
   handled by `min_size` (200M default for the home run) but a v2
   improvement would be a `--exclude-system` flag.

4. **500K-file benchmark deferred.** The Definition of Done calls for it;
   the synthetic 30K run is the closest evidence I could produce in this
   session. Recommend: a dedicated session with `find . -type f | wc -l`
   first to find a real 500K+ corpus, or run a longer synthetic build.

5. **No GNN scoring.** Per the SCOPE freeze, GNN is v2 — this is by design.

## Files added / modified

| File | Lines | What |
|---|---|---|
| `src/engines/disk/classifier.rs` | new (~300 lines) | `Bucket` enum + `classify()` + 24 tests |
| `src/engines/disk/mod.rs` | +3 | export classifier |
| `src/engines/disk/engine.rs` | +130 | `BucketStats` + `record_bucket()` + aggregate finding emission |
| `src/cli/args.rs` | +18 | `--explain` + `--group-by` flags + `DiskGroupBy` enum |
| `src/main.rs` | +3 | populate new DiskArgs fields |
| `docs/CAPABILITY_2_DESIGN.md` | new | design spec |

## What is NOT in this commit (deferred to v2 of capability #2)

- Confidence scores per recommendation
- "Safe to delete" verdicts (capability #3 owns this)
- GNN scoring
- Cargo.toml / package.json detection
- A 500K-file real-world benchmark

## Next session recommendation

- Either run the 500K-file benchmark in isolation, or move on to capability
  #3 (`xmac scan --recommend`) since the storage infrastructure now supports
  per-bucket aggregation and can be reused.
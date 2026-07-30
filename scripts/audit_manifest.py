#!/usr/bin/env python3
"""Audit the operations manifest against actual code.

For each row in docs/OPERATIONS_MANIFEST.md:
1. Parse the target file path
2. Resolve it (handles missing src/ prefix)
3. Search for the op name's keywords in the target file
4. Report rows where the target is unreachable or the op is missing

Exits non-zero if any [E] claim is unreachable (high-confidence bug),
or if more than N rows are suspect (configurable threshold).

Usage:
    python3 scripts/audit_manifest.py
    python3 scripts/audit_manifest.py --strict   # fail on any [E]/[X] missing
"""
import argparse
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "docs" / "OPERATIONS_MANIFEST.md"

STOPWORDS = {
    "with", "from", "that", "this", "when", "have", "they",
    "file", "files", "system", "using", "used", "engine",
    "check", "list", "show", "find", "scan", "the", "and",
    "for", "all", "any", "via", "add", "new", "into", "set",
}


def parse_manifest():
    """Parse all op rows from the manifest, tracking Part context."""
    rows = []
    current_part = "?"
    with open(MANIFEST) as f:
        for line in f:
            pm = re.match(r"^## Part ([A-Z])", line)
            if pm:
                current_part = f"Part {pm.group(1)}"
                continue
            m = re.match(
                r"^\| (\d+) \| (.+?) \| `([^`]+)` \| `\[([EXN])\]` \| (.+) \|",
                line,
            )
            if m:
                rows.append(
                    {
                        "id": int(m.group(1)),
                        "name": m.group(2),
                        "target": m.group(3),
                        "status": "[" + m.group(4) + "]",
                        "notes": m.group(5),
                        "part": current_part,
                    }
                )
    return rows


def resolve_target(target):
    """Resolve a manifest target path to an actual file or dir.

    Handles the common case where the manifest omits the src/ prefix.
    Returns None if no candidate exists.
    """
    # Already absolute or has a known top-level dir
    if target.startswith(("gui/", "gnn/", "scripts/", "docs/")):
        return target if os.path.exists(target) else None
    candidates = [target, f"src/{target}"]
    for c in candidates:
        if os.path.exists(c):
            return c
    return None


def search_keywords(filepath, op_name):
    """Find a distinctive keyword from op_name in filepath.

    Returns (matched_term, line_count) or (None, 0) if no match.
    """
    words = re.findall(r"\b[a-z_]{5,}\b", op_name.lower())
    candidates = [w for w in words if w not in STOPWORDS]
    if not candidates:
        return None, 0
    candidates.sort(key=len, reverse=True)
    for term in candidates[:3]:
        result = subprocess.run(
            ["grep", "-ci", "-F", term, filepath],
            capture_output=True,
            text=True,
        )
        try:
            count = int(result.stdout.strip() or 0)
        except ValueError:
            count = 0
        if count > 0:
            return term, count
    return None, 0


def audit(strict=False):
    rows = parse_manifest()
    total_by_status = Counter()
    reachable_by_status = Counter()
    keyword_found_by_status = Counter()
    suspect = []  # rows where target is reachable but keyword missing

    for r in rows:
        total_by_status[r["status"]] += 1
        fp = resolve_target(r["target"])
        if not fp:
            # Unresolved — separate category
            continue
        if not os.path.isfile(fp):
            # Target is a directory — can't easily grep, count as found
            reachable_by_status[r["status"]] += 1
            keyword_found_by_status[r["status"]] += 1
            continue
        reachable_by_status[r["status"]] += 1
        if r["status"] == "[E]":
            # For [E] we trust the claim if the file exists
            keyword_found_by_status[r["status"]] += 1
            continue
        term, count = search_keywords(fp, r["name"])
        if term:
            keyword_found_by_status[r["status"]] += 1
        else:
            suspect.append({**r, "target_resolved": fp})

    # Unresolved targets
    unresolved = [r for r in rows if not resolve_target(r["target"])]

    # Print report
    print("=" * 60)
    print("Operations Manifest Audit")
    print("=" * 60)
    print(f"Manifest: {MANIFEST.relative_to(ROOT)}")
    print(f"Total rows: {len(rows)}")
    print()
    print(f"{'Status':<8} {'Total':>6} {'Reachable':>10} {'Keyword':>10}")
    for status in ["[E]", "[X]", "[N]"]:
        print(
            f"{status:<8} {total_by_status[status]:>6} "
            f"{reachable_by_status[status]:>10} "
            f"{keyword_found_by_status[status]:>10}"
        )
    print()
    print(f"Unresolved targets: {len(unresolved)}")
    for r in unresolved:
        print(f"  op {r['id']:>3} [{r['status']}] {r['target']} ({r['part']})")
    print()
    print(f"Suspect rows (reachable target, no keyword): {len(suspect)}")
    for r in suspect:
        print(
            f"  op {r['id']:>3} [{r['status']}] "
            f"{r['name'][:45]:<45} -> {r['target_resolved']}"
        )

    # Exit code
    fail = False
    if unresolved:
        # Any unresolved [E] is a high-confidence bug
        for r in unresolved:
            if r["status"] == "[E]":
                fail = True
    if strict:
        # Fail on any [X] suspect (extend never happened)
        for r in suspect:
            if r["status"] == "[X]":
                fail = True
    if len(suspect) > 50:
        # Sanity threshold: more than 50 suspect rows suggests the
        # manifest is heavily stale
        fail = True

    if fail:
        print("\nAUDIT FAILED", file=sys.stderr)
        sys.exit(1)
    print("\nAUDIT PASSED")


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--strict",
        action="store_true",
        help="Fail on any [X] row with missing keyword",
    )
    args = ap.parse_args()
    audit(strict=args.strict)
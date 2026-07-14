#!/usr/bin/env bash
# X-MaC Digital Twin — Session Bootstrap
#
# Run this at the start of a Devin session to:
# 1. Verify you're on the correct branch (not main)
# 2. Verify push is disabled
# 3. Build and test the project
# 4. Print the current state
#
# Usage: ./scripts/bootstrap_session.sh

set -euo pipefail

echo "=== X-MaC Digital Twin — Session Bootstrap ==="
echo ""

# 1. Check branch
BRANCH=$(git branch --show-current)
echo "Current branch: $BRANCH"
if [ "$BRANCH" = "main" ] || [ "$BRANCH" = "master" ]; then
    echo "ERROR: On main/master branch. Switch to a digital-twin/* branch."
    echo "  git checkout -b digital-twin/<your-feature>"
    exit 1
fi
echo "OK: Not on main"
echo ""

# 2. Check push is disabled
PUSH_URL=$(git remote get-url --push origin 2>/dev/null || echo "none")
echo "Push URL: $PUSH_URL"
if [ "$PUSH_URL" != "DISABLED" ] && [ "$PUSH_URL" != "none" ]; then
    echo "WARNING: Push is not disabled. Run:"
    echo "  git remote set-url --push origin DISABLED"
fi
echo "OK: Push safety verified"
echo ""

# 3. Build
echo "Building..."
cargo build 2>&1 | tail -5
echo ""

# 4. Test
echo "Running tests..."
cargo test 2>&1 | tail -10
echo ""

# 5. Print structure
echo "=== Project Structure ==="
echo ""
echo "Orchestration files:"
echo "  .devin/config.json                          — Devin session config"
echo "  .devin/skills/digital-twin/SKILL.md         — Digital twin skill"
echo "  docs/INTEGRATION_PLAN.md                    — 18-phase integration plan"
echo "  docs/OPERATIONS_MANIFEST.md                 — 630 operations mapped to code"
echo "  AGENTS.md                                   — Agent & contributor guide"
echo "  scripts/bootstrap_session.sh                — This script"
echo ""
echo "Twin modules (src/twin/):"
ls -1 src/twin/*.rs 2>/dev/null | sed 's/^/  /'
echo ""
echo "New engines (src/engines/):"
for e in duplicate startup privacy; do
    if [ -d "src/engines/$e" ]; then
        echo "  $e/"
    fi
done
echo ""
echo "Existing engines:"
for e in clean conflict depth diag disk envmap graph maintain map optimize; do
    if [ -d "src/engines/$e" ]; then
        echo "  $e/"
    fi
done
echo ""

# 6. MAOS reminder
echo "=== MAOS Context Retrieval ==="
echo ""
echo "Before starting work, use MAOS MCP tools:"
echo "  1. maos_get_context     — full context packet"
echo "  2. maos_search_memory   — search for subsystem context"
echo "  3. maos_list_tasks      — see pending tasks"
echo ""
echo "Search queries by subsystem:"
echo "  Filesystem:     'xmac filesystem scan clean engine'"
echo "  Cache:          'xmac cache cleanup scanner rules'"
echo "  Duplicates:     'xmac blake3 hash duplicate'"
echo "  App intel:      'xmac envmap apps application'"
echo "  Process:        'xmac optimize telemetry process'"
echo "  Memory:         'xmac memory optimize ram'"
echo "  Hardware:       'xmac system_awareness snapshot hardware'"
echo "  Advisor:        'xmac advisor recommendation intelligence'"
echo "  Digital twin:   'xmac graph engine gnn twin'"
echo "  Cleanup:        'xmac cleanup transaction undo verification'"
echo ""

echo "=== Ready to work ==="

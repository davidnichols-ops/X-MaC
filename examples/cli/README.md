# CLI Usage Examples

## Basic Usage

```bash
# Quick overview — clean scan + maintenance + disk breakdown
xmac quick

# Find reclaimable space (no deletion)
xmac clean

# Find and delete with confirmation
xmac purge

# Disk usage breakdown
xmac disk

# Run maintenance tasks
xmac maintain
```

## AI Advisor

```bash
# Get system health recommendations
xmac advisor

# Top 3 recommendations only
xmac advisor --top 3

# JSON output for scripting
xmac --format json advisor
```

## Zen Mode

```bash
# Preview what Zen Mode would do (no changes)
xmac zen --no-clean --no-maintain

# Execute full optimization
xmac zen

# Skip disk cleanup, only maintenance + memory
xmac zen --no-clean

# JSON output
xmac --format json zen --no-clean --no-maintain
```

## Config Management

```bash
# Initialize default config
xmac config init

# List available profiles
xmac config profiles

# Switch profile
xmac config set-profile gaming

# Get a specific setting
xmac config get clean.min_age_days

# Set a specific setting
xmac config set clean.min_age_days 7

# Show full config
xmac config show
```

## Daemon

```bash
# Start daemon in foreground
xmac daemon

# Start daemon in background
xmac daemon --start

# Check daemon status
xmac daemon --status

# Stop daemon
xmac daemon --stop

# Verbose logging
xmac daemon --verbose
```

## History

```bash
# Show scan history
xmac history

# Summary statistics
xmac history --summary

# Export history as JSON
xmac history --export history.json

# Clear history
xmac history --clear
```

## Output Formats

```bash
# Human-readable text (default)
xmac clean

# JSON (single object)
xmac --format json clean

# NDJSON (streaming, one finding per line — for GUI integration)
xmac --format ndjson clean
```

## Combining with Shell Tools

```bash
# Count findings by category
xmac --format json clean | jq '.findings | group_by(.category) | map({category: .[0].category, count: length})'

# Find the 10 largest reclaimable items
xmac --format json clean | jq '[.findings[] | select(.size_bytes != null)] | sort_by(.size_bytes) | reverse | .[:10] | map({title, size_bytes, category})'

# Export findings to CSV
xmac --format json clean | jq -r '.findings[] | [.title, .category, .severity, (.size_bytes // 0)] | @csv'
```

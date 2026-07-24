# X-MaC GUI Design Brief

> **For:** Graphics design AI tasked with reworking the X-MaC app GUI
> **App:** X-MaC — The Intelligence Layer Above macOS
> **Version:** 2.1.1
> **Platform:** macOS 14+ (Sonoma), Apple Silicon
> **Language:** Swift / SwiftUI
> **Bundle ID:** com.xmac.gui
> **License:** MIT

---

## 1. What X-MaC Is

X-MaC is a macOS system intelligence tool — not just a cleaner. It combines:

- **13 scanning engines** (clean, disk, depth, diag, envmap, graph, maintain, map, optimize, conflict, duplicate, startup, privacy)
- **Digital Twin** — a live computational model of the Mac (hardware, software, filesystem, processes, memory, energy, apps) backed by a SQLite event store with real-time observers
- **GNN predictions** — a trained Graph Attention Network (CoreML, <1ms inference) that predicts memory pressure per-process with 97% action accuracy
- **Safety rules** — 38 YAML rules with three-tier classification (safe/review/protected) that govern every cleanup action
- **MCP server** — exposes the Digital Twin to AI agents (Claude, Cursor) with read-only vs destructive tool split and bearer-token auth
- **"What Changed?"** — temporal queries against the event store ("show me what changed in the last 24 hours")

The app is **dark-first**, with a "neural" aesthetic — electric cyan accents on deep navy voids, metallic silver text, and glow effects throughout. Think: a mission control dashboard for your Mac, not a toy cleaner.

---

## 2. App Architecture

### Window
- **Min size:** 1100 × 720pt
- **Style:** `.titlebar` (standard macOS title bar)
- **Color scheme:** User-configurable (system/light/dark), defaults to dark

### Navigation
- `NavigationSplitView` with sidebar (220–240pt) + detail pane
- **10 main tabs** in the sidebar, some with nested sub-items
- Sub-tabs within tabs use a horizontal pill-style picker at the top

### Menu Bar Extra
- Icon: `cpu` (SF Symbol) in accent color
- Shows scan status or system health score
- Quick actions: Open, Quick Clean, Zen Mode, AI Advisor, Quit

### Keyboard Shortcuts
- Cmd+Shift+F: Full Scan
- Cmd+Shift+N: Neural Scan
- Cmd+Shift+Z: Zen Mode Preview
- Cmd+Shift+A: AI Advisor Analyze

---

## 3. Design System (XTheme.swift)

### Color Palette

| Token | Hex | RGB | Usage |
|-------|-----|-----|-------|
| `bgVoid` | #000810 | 0, 3, 6 | Deepest background |
| `bgPrimary` | #001020 | 0, 6, 13 | Main background |
| `bgSecondary` | #0A1525 | 4, 8, 15 | Sidebar, panels |
| `bgTertiary` | #102030 | 6, 13, 19 | Cards, raised surfaces |
| `accent` | #57CAFF | 34, 79, 100 | Primary accent (electric cyan) |
| `accentBright` | #3FB8FF | 25, 72, 100 | Hover/active states |
| `accentDim` | #1A6090 | 10, 38, 56 | Inactive/disabled |
| `neuralBlue` | #1A2F4E | 10, 18, 31 | Dark accent for neural UI |
| `metallic` | #E8E8E8 | 91, 91, 91 | Primary text, X logo |
| `metallicWarm` | #F0E7DF | 94, 91, 87 | Metallic gradient endpoint |
| `steel` | #BFBFBE | 75, 75, 75 | Secondary metallic |
| `teal` | #4288A8 | 26, 53, 66 | Secondary accent |
| `tealDeep` | #29546B | 16, 33, 42 | Dark teal accent |
| `info` | #57CAFF | 34, 79, 100 | Info severity |
| `low` | #4DCC88 | 30, 80, 55 | Low severity (teal-green) |
| `medium` | #F2BF40 | 95, 75, 25 | Medium severity (amber) |
| `high` | #F28040 | 95, 50, 25 | High severity (orange) |
| `critical` | #E64852 | 90, 28, 32 | Critical severity (red) |
| `safe` | #33D185 | 20, 82, 52 | GNN safe / safety safe |
| `warning` | #F2BF40 | 95, 75, 25 | GNN warning |
| `danger` | #E64D59 | 90, 30, 35 | GNN danger / safety protected |
| `anomaly` | #8C66F2 | 55, 40, 95 | Neural anomaly (purple) |
| `textPrimary` | #E8E8E8 | 91, 91, 91 | Primary text |
| `textSecondary` | #949EA8 | 58, 62, 68 | Secondary text |
| `textTertiary` | #616B78 | 38, 42, 48 | Tertiary/muted text |
| `cardBg` | #0D1A28 | 5, 10, 16 | Card background |
| `cardBorder` | #1F3346 | 12, 20, 28 | Card border (cyan-tinted) |

### Gradients

| Token | Direction | Stops | Usage |
|-------|-----------|-------|-------|
| `neuralGradient` | Linear (leading→trailing) | accent → teal | Primary buttons, hero text |
| `hudGradient` | Linear (topLeading→bottomTrailing) | accentBright → #1A66CC | HUD elements |
| `metallicGradient` | Linear (top→bottom) | metallic → metallicWarm | Logo, large text |
| `voidGradient` | Linear (top→bottom) | bgVoid → bgPrimary | Main background |
| `cardGradient` | Linear (top→bottom) | cardBg → bgSecondary | Card fills |
| `sidebarGradient` | Linear (top→bottom) | bgSecondary → bgVoid | Sidebar |

### Spacing & Sizing

| Element | Value |
|---------|-------|
| XCard padding | 16pt |
| XCard corner radius | 12pt |
| XCard border width | 1pt |
| XCard shadow | accent.opacity(0.04), radius 6 |
| Section spacing | 16–20pt |
| Within-card spacing | 8–12pt |
| Sub-tab corner radius | 6pt |
| Sub-tab padding | horizontal 12, vertical 6 |
| NavButton corner radius | 8pt |
| NavButton padding | horizontal 12, vertical 8 |
| NavButton indent per level | 16pt |
| Search bar corner radius | 8pt |
| Safety badge corner radius | 4pt |

### Glow Effects

| Effect | Opacity | Radius | Offset |
|--------|---------|--------|--------|
| `xGlow()` | 0.35 | 8pt | default |
| `xHeroGlow()` | 0.40 | 14pt | x:0, y:2 |
| NavButton active | — | 3pt | — |
| Hero icon | — | 6–16pt | — |

### Typography

| Context | Size | Weight | Design |
|---------|------|--------|--------|
| Hero numbers | 48pt | bold | rounded |
| Hero titles | 28–34pt | bold | rounded |
| Large stats | 32–36pt | bold | default |
| Section headers | 14pt | semibold | default |
| NavButton text | 13pt | semibold (active) / regular (inactive) | default |
| Body text | 12–13pt | regular | default |
| Small text / labels | 10–11pt | medium | default |
| Monospaced (paths, code, data) | 9–11pt | regular | monospaced |
| Logo | 18pt | bold | rounded |
| Onboarding title | 28–34pt | bold | default |
| Onboarding body | 14pt | regular | default |

### Iconography
- **SF Symbols exclusively** — no custom icon assets
- Icon sizes: 10pt (badges), 14pt (nav/headers), 16pt (settings), 20–24pt (cards), 40–56pt (hero/empty states), 72pt (onboarding welcome)
- Active icons get glow effects

### App Icon
- `AppIcon.icns` — 1024×1024, 2.5MB
- Currently a custom design (not using asset catalog)
- No other image assets in the app

---

## 4. Sidebar Structure

### Logo
- Icon: `cpu` at 24pt bold, metallic gradient + glow (radius 6)
- Text: "X-MaC" at 18pt bold, rounded design, metallic gradient
- Padding: vertical 16pt

### 10 Main Tabs

| # | Tab | Icon | Has Sub-items? |
|---|-----|------|----------------|
| 1 | Overview | `rectangle.grid.2x2` | No |
| 2 | System | `cpu.fill` | Yes: Hardware, Software, Memory, Energy |
| 3 | Applications | `app.badge` | No |
| 4 | Filesystem | `internaldrive` | Yes: Clean, FS Integrity, Conflicts, Env Map |
| 5 | Activity | `chart.bar.fill` | Yes: Processes, RAM Boost |
| 6 | Optimization | `wand.and.stars` | Yes: Zen Mode, AI Advisor, Reasoning, Quick Scan, Purge, History, Config |
| 7 | Intelligence | `brain.fill` | No |
| 8 | Timeline | `clock.fill` | No (sub-tabs in detail pane) |
| 9 | Automation | `gearshape.arrow.triangle.2.circlepath` | No (sub-tabs in detail pane) |
| 10 | Assistant | `bubble.left.and.bubble.right.fill` | No |

### Bottom Section
- Diagnostics (`stethoscope`)
- Settings (`gearshape`)

### NavButton Styling
- Icon: 14pt medium, width 20pt
- Text: 13pt (semibold if active, regular if inactive)
- Padding: horizontal 12, vertical 8
- Indent: 16pt per nesting level
- Corner radius: 8pt
- Active background: LinearGradient accent.opacity(0.15) → accent.opacity(0.05)
- Active border: accent.opacity(0.2), 1pt
- Active glow: accent, radius 3

---

## 5. Sub-Tab System

Sub-tabs appear as a horizontal pill picker at the top of the detail pane.

### Filesystem Sub-Tabs
| Label | Icon | View |
|-------|------|------|
| Disk Usage | `internaldrive` | DiskView |
| Clean | `trash.circle` | CleanView |
| FS Integrity | `checkmark.shield` | DepthView |
| Depth Scan | `scope` | DepthView |
| Safety | `shield.lefthalf.filled` | SafetyView |

### Activity Sub-Tabs
| Label | Icon | View |
|-------|------|------|
| Processes | `gearshape.2.fill` | TwinProcessView |
| Memory | `memorychip.fill` | TwinMemoryView |
| RAM Boost | `bolt.fill` | RamBoostView |
| Energy | `flame.fill` | TwinEnergyView |

### Optimization Sub-Tabs
| Label | Icon | View |
|-------|------|------|
| Zen Mode | `circle.hexagongrid` | ZenView |
| AI Advisor | `brain.head.profile` | AdvisorView |
| Reasoning | `lightbulb.fill` | TwinReasoningView |
| Purge | `trash.fill` | PurgeView |
| Quick Scan | `bolt.circle` | QuickScanView |

### Timeline Sub-Tabs
| Label | Icon | View |
|-------|------|------|
| Event Timeline | `timeline` | TimelineView |
| What Changed? | `arrow.triangle.2.circlepath` | WhatChangedView |

### Automation Sub-Tabs
| Label | Icon | View |
|-------|------|------|
| Automation | `gearshape.arrow.triangle.2.circlepath` | AutomationView |
| Twin Management | `externaldrive.connected.to.line.below` | TwinManagementView |

### Sub-Tab Styling
- Font: 11pt icon, 12pt text, medium weight
- Padding: horizontal 12, vertical 6
- Corner radius: 6pt
- Active background: accent.opacity(0.2)

---

## 6. Every Screen — Detailed Breakdown

### 6.1 Overview (Mission Control)
**Purpose:** Landing page with system health scores and alerts

**Layout:**
- 6 score cards in 3-column grid (System Health, Trust Score, Memory, Storage, Battery, Security)
- Each score card: circular progress indicator (50×50pt), colored by score value
- Active alerts section (critical/high severity only)
- AI recommendations section with workflow changes and preventive actions
- Recent timeline with process anomalies, memory leaks, storage warnings

**Score Card Colors:**
- 85+ → safe (green)
- 70+ → accent (cyan)
- 50+ → medium (amber)
- 30+ → high (orange)
- <30 → critical (red)

### 6.2 Dashboard
**Purpose:** Quick actions and reclaimable space hero

**Layout:**
- Hero: Large reclaimable space number (48pt bold, rounded, metallic gradient)
- Primary button: "Quick Clean" (280×52, neural gradient, capsule, glow radius 6)
- Secondary buttons: "Neural Scan" and "Full Scan" (180×44, bordered, corner radius 10)
- 4-column metric grid: Reclaimable, Findings, Last scan, Neural safe
- Recommendation card with lightbulb icon
- Protection card with 3 feature rows

### 6.3 Digital Twin Dashboard
**Purpose:** Main twin view with health scores and navigation to sub-views

**Layout:**
- Header: "Digital Twin" (24pt bold, rounded, metallic gradient) + Refresh button
- 4 score cards in HStack: System Health, Trust Score, Total Processes, Total Apps
- 8 navigation cards in 2-column grid: Hardware, Software Genome, Filesystem, Processes, Memory, Energy, App Intelligence, Reasoning Engine
- Each nav card: icon (22pt), title (13pt semibold), subtitle (11pt), chevron
- Anomalies preview, memory leaks preview, suspicious apps preview

**States:** Loading (ProgressView), Error (octagon icon), Empty (brain icon)

### 6.4 Twin Sub-Views (8 screens)
**Purpose:** Detailed drill-down into each Digital Twin dimension

**Hardware:** Machine identity, CPU topology, GPU + Neural Engine (side by side), Memory, Storage (list of drives with SMART health), Battery, Thermal, Power state, Peripherals

**Software Genome:** Total components (32pt large), applications list (top 20 by size), 18 component category cards in 3-column grid

**Filesystem:** 4 stat cards (Total Size, Total Files, Growth Trend, Days to Full), duplicate clusters, abandoned/orphan files

**Processes:** Total processes large display, top CPU consumers, top memory consumers, anomalies

**Memory:** Memory stats with gauge, top consumers, leak candidates, fragmentation

**Energy:** Battery stats, energy consumers, thermal efficiency, sleep/wake causes

**App Intelligence:** App list, suspicious apps, unused apps, duplicate apps

**Reasoning:** Ask question input, recommended actions, simulation results, sandbox results

### 6.5 Clean View
**Purpose:** Clean scan results with safety classifications and cleanup actions

**Layout:**
- HStack: CleanReclaimCard (large reclaimable display) + CleanSafetySummaryCard (4 safety stats + horizontal bar chart)
- CleanCategoryBreakdown: Checkable category list with progress bars (8pt height)
- SearchBar
- CleanupToolbar: "Select safe items", "Move to Trash" (with confirmation dialog), "Clear"
- CleanFindingsList: Grouped by category, each row is SelectableFindingRow

**Safety Summary Card:**
- 4 stats: Safe (green), Review (amber), Protected (red), Unclassified (gray)
- Horizontal stacked bar chart showing distribution

**SelectableFindingRow:**
- Checkbox (checkmark.square.fill vs square, 16pt, green when selected)
- Severity icon
- Title, path (FilePathDisplay, monospaced 9pt, truncated middle), description
- SafetyBadge (if classified): shield icon + rating text + confidence %, color-coded, 4pt corner radius

### 6.6 Disk View
**Purpose:** Disk usage with interactive donut chart

**Layout:**
- HStack: Interactive donut chart (220×220, 34pt thickness) + legend
- VolumeStatPills: Total, Used, Free
- DiskBreakdownList: Directories and large files

**Donut Chart:**
- Canvas-based rendering with animated fill (0.8s easeOut)
- 0.012 rad gap between segments
- Hover: segment expands by 5pt outer radius, center label updates
- Palette: cyan, teal, green, amber, orange, bright cyan, purple
- `onContinuousHover` for mouse tracking with angle-based hit testing

### 6.7 RAM Boost View
**Purpose:** Memory optimization with GNN predictions

**Layout:**
- GNN pressure banner: Large icon (28pt) with glow, NEURAL/HEURISTIC badge
- Memory dashboard: Circular gauge (90×90, 10pt stroke) + stats
- GNN predictions: Actionable items with toggle, process list with Kill/Suspend buttons
- Boost controls: Purge + kill options
- Boost result: Before/after comparison

**Process Row Colors:** terminate (red), suspend (orange), no_action (gray)
**Auto-refresh:** 10s interval

### 6.8 Zen Mode
**Purpose:** One-click comprehensive optimization

**Layout:**
- Hero: Hexagon icon (48pt, xHeroGlow radius 16), "Zen Mode" title (32pt)
- Two buttons: Preview (140×48) and Optimize Now (180×48, neural gradient, capsule)
- 4 feature rows explaining what Zen does
- Results: Health score (before/after/change, 36pt bold), Memory (usage + gauge 56×56), Disk (reclaimed + categories), Steps (checklist)

### 6.9 AI Advisor
**Purpose:** AI-powered system recommendations in natural language

**Layout:**
- Hero: Brain head icon (48pt, xHeroGlow radius 16, anomaly color), "AI Advisor" title (32pt)
- "Analyze System" button (220×52)
- Results: Health gauge (80×80, 8pt stroke) + status, recommendation cards

**Recommendation Card:**
- Severity icon, confidence badge, title, explanation, action, command (monospaced)
- Colors: severity-based

### 6.10 Neural Scan
**Purpose:** GNN-powered filesystem analysis

**Layout:**
- Idle: Brain icon (56pt), feature list, Start button
- Scanning: Rotating circle animation (2s linear), network icon (40pt), progress bar, Stop button
- Results: 8-stat summary, TabView (Scores tab, Smart Purge tab)
- Scores: Sorted list with safety/anomaly/confidence bars (5pt height, 3pt corner radius)
- Smart Purge: Confidence card, cleanup toolbar, purge order, anomaly hotspots, cross-directory patterns

### 6.11 What Changed?
**Purpose:** Temporal diff showing system changes over time

**Layout:**
- Header: Time range picker (1h, 6h, 24h, 7d, 30d, menu style, 160px) + Refresh
- Summary card: 4 stats (New Apps, Removed, File Changes, Alerts)
- Sections: New/removed apps, storage growth, process changes, major file changes, security alerts, config changes, timeline

**Colors:** Green for created/new, red for deleted/alerts, orange for review

### 6.12 Safety View
**Purpose:** Safety rules browser and path classifier

**Layout:**
- Summary card: 3 stats (Safe, Review, Protected) with counts
- Rules sections grouped by rating, each rule shows: name, description, confidence, category, paths
- Path classifier: Monospaced text field (12pt, 8pt corner radius, bgTertiary) + Classify button
- Classification result: Large rating icon (20pt) with glow, rule name, description, confidence, explanation

**Rule row indicators:** Small circle dots (8pt) with glow, color-coded by rating

### 6.13 Twin Management
**Purpose:** Database lifecycle and observer controls

**Layout:**
- Database card: Init DB button, Compact DB button, status indicator (checkmark circle or pulsing circle with glow)
- Observer card: Duration picker (10s/30s/1m/5m/30m, menu style 160px), Start Observers button, isObserving indicator
- What Changed quick access: Button that navigates to WhatChangedView

### 6.14 Automation
**Purpose:** Scheduled scans and launch agent management

**Layout:**
- Policy card: 3 toggles (auto-scan, notify, quiet hours), hour pickers (00-23, menu style 80px)
- Schedule card: Daily scan toggle, LaunchAgent status indicator (8×8 circle)
- Action buttons: Install LaunchAgent, Uninstall, Check Status
- Message display (monospaced 11pt)

### 6.15 App Inventory
**Purpose:** Browse installed apps with footprint analysis

**Layout:**
- Header: App count, total footprint, Rescan + Move to Trash buttons
- Search bar + sort buttons (Size, Name, Kind)
- HStack: App list (320–360px, sidebar gradient bg) + detail panel (420px min)
- App list row: Checkbox, app icon, name, bundle ID, size
- Detail panel: Bundle size, related paths grouped by kind, Reveal in Finder, Trash buttons

**Colors:** System apps (textTertiary), user apps (accent)

### 6.16 Settings
**Purpose:** App configuration

**Sections:**
1. Binary — xmac path display
2. Cleanup Profiles — picker, duplicate/export/import/delete
3. Per-category policy — 18 categories, segmented picker (trash/review/blocked), 2-column grid
4. Cleanup policy — Toggles (confirm, trash, hidden, symlinks), Steppers (age 1-365d, size 0-10000MB)
5. Appearance — Theme picker (system/light/dark, segmented)
6. Exclusions — TextEditor (monospaced 11pt, 90pt min height)
7. CoreML Model — Load status
8. About — App info, version, GNN accuracy
9. What's New — Feature list with icons

### 6.17 Onboarding (5 pages)
**Purpose:** First-run experience

**Pages:**
1. "Meet X-MaC" — shield.checkered icon (72pt), accent colors
2. "One-tap clean, zero surprises" — sparkles icon (56pt), green colors
3. "GNN scoring, on your device" — brain icon (56pt), purple colors
4. "Nothing moves without you" — hand.raised.fill icon (56pt), amber/orange colors
5. "You're all set" — checkmark.circle.fill icon (56pt), green colors

**Visual effects:**
- 6 floating particles (3-5pt, white 15% opacity, blur 0.5, 3.5-4.5s duration)
- Radial glow background (blue gradient, 60% width radius)
- Page dots: 8pt circles, spacing 12pt, scale 1.25× when active
- Buttons: Capsule, neural gradient, 44-48pt height
- Spring transitions: 0.5s response, 0.85 damping

### 6.18 Diagnostics
**Purpose:** Run every CLI command and verify JSON output

**Layout:**
- Header with pass/fail counts, Run All / Clear buttons
- 9 category sections, each with command rows
- Command row: Status icon, command name, CLI args, description, Run button, result summary
- Expandable raw output: Exit code, duration, JSON validity, finding count, output preview (black bg 30% opacity, monospaced 10pt)

### 6.19 Scan History
**Purpose:** Track scan history and growth trends

**Layout:**
- Summary: 3 stats (Scans, Tracked reclaimable, Avg duration)
- Growth analysis: Reclaimable change, finding change, latest scan date
- History list: Timestamp, mode, reclaimable, findings, duration
- Context menu: Re-run scan, export to clipboard
- Change badges: Arrow icon + value, color-coded (up=warning, down=safe)

### 6.20 Missing/Placeholder Views
These views exist but are simpler implementations:
- **ConflictView** — PATH conflicts, env var conflicts, port usage
- **EnvMapView** — System environment, packages, apps
- **DepthView** — Filesystem integrity, permissions, symlinks
- **QuickScanView** — Composite clean + maintain + disk
- **PurgeView** — Cleanup execution with fix script generation
- **ConfigView** — Configuration profiles management

---

## 7. Reusable Components

| Component | Description | Key Styling |
|-----------|-------------|-------------|
| `XCard` | Card wrapper | 16pt padding, 12pt radius, 1pt border, cardGradient fill |
| `XSectionHeader` | Section title with icon + count badge | 14pt semibold, 8pt spacing |
| `SelectableFindingRow` | Finding row with checkbox + safety badge | 16pt checkbox, monospaced path |
| `SearchBar` | Search input with clear button | 8pt radius, bgTertiary, 10pt padding |
| `SafetyBadge` | Safety rating badge | 4pt radius, color-coded, shield icon |
| `StatBadge` | Icon + label + value | Used in summaries across views |
| `ScoreCard` | Circular gauge + percentage | 50×50pt, colored by score |
| `ScanProgressView` | Progress with phase text | Used in all scan views |
| `ScanErrorView` | Error display with dismiss | Octagon icon |
| `EmptyScanView` | Empty state | Large icon + prompt |
| `FilePathDisplay` | Truncated middle path | Monospaced 9pt |
| `NavButton` | Sidebar navigation button | See sidebar styling above |

---

## 8. Data Model (What the GUI displays)

### Finding
```
id, engine, severity, category, target (type + value),
title, description, metadata, discovered_at,
remediation_hint, size_bytes,
safety_rating, safety_explanation, safety_rule, safety_confidence
```

### Digital Twin
```
timestamp_ms
├── hardware (model, SoC, CPU cores, GPU, Neural Engine, memory, storage, battery, thermal, peripherals, power)
├── software_genome (apps, frameworks, dylibs, kexts, extensions, launch agents/daemons, plugins, fonts, dev tools, SDKs, package managers, Python/Node/Rust envs, Docker images, containers, VMs, AI models, datasets)
├── filesystem (total files, total size, duplicates, abandoned/orphan files, growth trend, exhaustion forecast)
├── processes (process tree, anomalies, bottlenecks, idle processes, crashed services)
├── memory (total/used/available/compressed/swap, utilization, pressure, top consumers, leak candidates, fragmentation)
├── energy (battery, energy consumers, thermal efficiency, sleep efficiency, wake causes)
├── applications (apps, suspicious apps, unused apps, duplicate apps)
├── health_score (0-100)
└── trust_score (0-100)
```

### What Changed Report
```
total_events
├── new_applications / removed_applications
├── storage_growth (bytes added/removed, net change, files created/deleted)
├── process_changes (launched, terminated, anomalies, top CPU/memory consumers)
├── major_file_changes
├── security_alerts
├── configuration_changes
└── timeline
```

### GNN Response
```
scores (path, label, safety_score, anomaly_score, confidence, explanation, size_bytes)
summary (total_files, safe/review/danger counts, avg safety/anomaly, potential reclaim bytes)
purge_plan (impact-weighted order, anomaly hotspots, cross-directory patterns)
```

### Safety Rules
```
total_rules, counts_by_rating
rules[]: name, description, rating (safe/review/protected), paths[], confidence, category, upstream_commit
```

---

## 9. Build & File Structure

### Source Files (41 Swift files)
```
gui/XMacApp/Sources/XMacApp/
├── XMacApp.swift          — @main entry, WindowGroup, MenuBarExtra
├── XTheme.swift           — Design system (colors, gradients, components)
├── ContentView.swift      — NavigationSplitView, sidebar, mode routing
├── XMacRunner.swift       — Main view model (CLI bridge, @Published state)
├── Models.swift           — Finding, GNN, Zen, Advisor models
├── TwinModels.swift       — Digital Twin Codable structs
├── TabContainerViews.swift — Sub-tab containers (5 tab groups)
├── OverviewIntelligenceViews.swift — Overview, Intelligence, Timeline
├── DashboardView.swift    — Quick actions dashboard
├── TwinDashboardView.swift — Digital Twin dashboard
├── TwinSubViews.swift     — 8 twin detail sub-views
├── TwinManagementView.swift — DB management + observer controls
├── CleanView.swift        — Clean scan results + safety summary
├── DiskView.swift         — Disk usage with donut chart
├── RamBoostView.swift     — Memory optimization with GNN
├── ZenView.swift          — One-click optimization
├── AdvisorView.swift      — AI Advisor recommendations
├── NeuralScanView.swift   — GNN filesystem analysis
├── SmartScanView.swift    — Rule vs GNN comparison
├── WhatChangedView.swift  — Temporal diff view
├── SafetyView.swift       — Safety rules + path classifier
├── AutomationView.swift   — Scheduled scans + launch agent
├── AppInventoryView.swift — App browser with footprint
├── ScanHistoryView.swift  — Scan history + growth trends
├── OnboardingView.swift   — 5-page first-run experience
├── SettingsView.swift     — App settings
├── DiagnosticsView.swift  — CLI command tester
├── SelectableFindingRow.swift — Finding row with safety badge
├── SearchBar.swift        — Reusable search input
├── MissingViews.swift     — Conflict, EnvMap, Depth, QuickScan, Purge, Config
├── FullScanView.swift     — Full scan results
├── MaintainView.swift     — Maintenance tasks
├── AppSettings.swift      — Settings data model
├── CleanupProfile.swift   — Cleanup profile model + store
├── PerCategoryPolicyEditor.swift — Category policy editor
├── CoreMLGNN.swift        — CoreML model manager
├── MemoryGNNManager.swift — Memory prediction model manager
├── ScanHistoryStore.swift — History persistence
├── XMacReportDocument.swift — Export document type
├── CrashReporter.swift    — Error logger
└── AdaptiveFixer.swift    — Auto error recovery
```

### Build
```bash
cd gui/XMacApp && swift build          # build Swift
cd gui && ./build_app.sh               # build full .app bundle
```

### Package.swift
- Swift 5.9, macOS 14+
- Single executable target "XMacApp"
- Resources: XMacMemoryGNN.mlpackage (CoreML model)

### No Asset Catalog
- All colors defined in code (XTheme.swift)
- SF Symbols for all icons
- Only image asset: AppIcon.icns (1024×1024)

---

## 10. Design Principles to Preserve

1. **Dark-first neural aesthetic** — deep navy voids, electric cyan, metallic silver, glow effects
2. **Safety-first** — every file shows its safety rating (safe/review/protected) with color-coded badges
3. **Data density without clutter** — monospaced fonts for technical data, clean cards for summaries
4. **Glow as hierarchy** — more important elements get stronger glow effects
5. **Color as communication** — green=safe, amber=review, red=protected/danger, purple=neural/AI, cyan=accent
6. **Trash-first, never permanent** — all cleanup goes to Trash with undo support
7. **Real-time where it matters** — observers, live process data, memory gauges with auto-refresh
8. **Temporal awareness** — "What Changed?" is a first-class view, not hidden in settings

## 11. Known Design Issues (Opportunities for the Designer)

1. **Redundant sub-tabs** — "FS Integrity" and "Depth Scan" both show DepthView
2. **35 ScanMode cases** — many are legacy duplicates (e.g., `.overview`, `.dashboard`, `.idle` all show OverviewView)
3. **Missing views are placeholders** — ConflictView, EnvMapView, DepthView, QuickScanView, PurgeView, ConfigView are basic
4. **No empty state illustration** — empty states use SF Symbols only, no custom illustrations
5. **No asset catalog** — all colors are in code, which makes theming harder for non-developers
6. **Onboarding is functional but not polished** — particle animation is simple, no video or rich graphics
7. **Donut chart is the only custom visualization** — no other charts (bar charts use simple progress bars)
8. **No light mode testing** — the app is designed dark-first; light mode may have contrast issues
9. **Sidebar nesting is inconsistent** — some tabs have nested items in sidebar, others use sub-tabs in detail pane
10. **No animations between sub-tabs** — switching sub-tabs is instant with no transition

# X-MaC GUI Implementation Report

## Repository Archaeology Results

### Current State
- **17,013 lines** of Swift across 42 files
- **Builds clean** (0 errors, 1 warning about missing mlpackage resource)
- **No tests** for GUI code
- **2 accessibility labels** in entire codebase

### Critical Issues Found

#### 1. Navigation: 35 ScanMode cases with heavy duplication
- `.overview`, `.dashboard`, `.idle` → all route to OverviewView
- `.applications`, `.apps` → both route to AppInventoryView
- `.system`, `.twin` → both route to TwinDashboardView
- `.twinFilesystem` vs `.filesystem` → inconsistent routing
- Sub-tabs duplicate: `.integrity` and `.depth` both → DepthView

#### 2. XMacRunner.swift: 1,671-line god object
- 58 @Published properties
- 98 methods
- Mixes scan lifecycle, cleanup, navigation, twin ops, safety, zen, advisor, RAM boost
- Multiple untracked Task launches (lines 332-392) — cannot cancel
- Progress animation task (lines 1001-1009) can orphan
- NotificationCenter observer (lines 105-117) never removed
- FileManager operations on @MainActor (lines 782, 842, 931, 967, 980, 993)
- No timeout on process execution (runProcess, line 1074)
- 5 separate finding arrays duplicate state (lines 71-78)

#### 3. Direct Process() calls in view layer (4 files)
- DiagnosticsView.swift:288
- MaintainView.swift:259
- RamBoostView.swift:977
- AutomationView.swift:145, 161 (blocking main thread)

#### 4. Thread safety: CoreML managers
- CoreMLGNN.swift: model/labelMap/reverseLabelMap unsynchronized
- MemoryGNNManager.swift: model unsynchronized
- Race conditions on concurrent predict() calls

#### 5. File I/O on @MainActor
- ProfileStore init/load/save (CleanupProfile.swift)
- ScanHistoryStore init/load/save
- CrashReporter writeEntry

#### 6. Destructive actions without confirmation
- ZenView: "Optimize Now" — no confirmation
- RamBoostView: process kill/suspend — no confirmation
- TwinManagementView: DB init/compact — no confirmation
- AutomationView: LaunchAgent uninstall — no confirmation
- ScanHistoryView: Clear History — no confirmation
- SettingsView: Delete profile — no confirmation

#### 7. Missing states (loading/empty/error)
- DashboardView: no loading, error, or empty states
- OverviewView: no loading or error states
- All 8 TwinSubViews: no loading or error states
- TwinManagementView: no loading, error, or empty states
- AutomationView: no loading or empty states
- SettingsView: no loading, error, or empty states
- ScanHistoryView: no loading or error states
- MissingViews: no loading or error states

#### 8. Hard-coded colors outside XTheme
- OnboardingView: 9 hard-coded Color literals
- DiagnosticsView: 1 (Color.black)
- OverviewIntelligenceViews: 2 (Color.gray, Color.white)
- DiskView: 2 (Color(white: 0.45), Color(white: 0.20))

#### 9. Silent error handling (try? everywhere)
- Models.swift: TargetInfo and JSONValue decoders silently fail
- CoreMLGNN.swift: 15+ silent failure points
- MemoryGNNManager.swift: 10+ silent failure points
- CleanupProfile.swift: all persistence operations use try?
- ScanHistoryStore.swift: all persistence operations use try?
- CrashReporter.swift: all file operations use try?

#### 10. .onAppear starting async work without cancellation
- 14 instances across 8 files
- No way to cancel operations when user navigates away

### What Must Not Regress
- CLI bridge (XMacRunner → xmac binary)
- CoreML model loading and inference
- Safety rule classification
- Cleanup flow (scan → select → trash)
- Digital Twin collection and display
- What Changed? queries
- Observer start/stop
- Zen Mode execution
- AI Advisor analysis
- App Inventory scanning
- Settings persistence
- Scan history persistence
- Onboarding flow
- Menu bar extra

### File Size Distribution (lines)
- XMacRunner.swift: 1,671 (god object)
- TwinSubViews.swift: 1,402 (8 views in one file)
- RamBoostView.swift: 1,050
- DiskView.swift: 878
- OverviewIntelligenceViews.swift: 816
- WhatChangedView.swift: 788
- NeuralScanView.swift: 753
- DiagnosticsView.swift: 696
- AppInventoryView.swift: 677
- ContentView.swift: 509
- All others: <500

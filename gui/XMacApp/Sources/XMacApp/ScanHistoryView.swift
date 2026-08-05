import SwiftUI

struct ScanHistoryView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if runner.historyStore.entries.isEmpty {
                    EmptyScanView(message: "No scan history yet. Run a scan to start tracking.")
                } else {
                    HistorySummary(entries: runner.historyStore.entries)
                    GrowthAnalysis(entries: runner.historyStore.entries)
                    HistoryList(entries: runner.historyStore.entries)
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }
}

private struct HistorySummary: View {
    let entries: [ScanHistoryEntry]

    private var totalScanned: Int { entries.count }
    private var totalReclaimed: Int { entries.map(\.reclaimableBytes).reduce(0, +) }
    private var avgDuration: Double {
        entries.isEmpty ? 0 : entries.map(\.durationSeconds).reduce(0, +) / Double(entries.count)
    }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Scan History", icon: "clock.arrow.circlepath")
                HStack(spacing: 24) {
                    StatBadge(icon: "number", label: "Scans", value: "\(totalScanned)", color: XTheme.accent)
                    StatBadge(icon: "arrow.down.circle", label: "Tracked reclaimable", value: formatBytes(totalReclaimed), color: XTheme.safe)
                    StatBadge(icon: "clock", label: "Avg duration", value: String(format: "%.1fs", avgDuration), color: XTheme.info)
                }
            }
        }
    }
}

private struct GrowthAnalysis: View {
    let entries: [ScanHistoryEntry]

    private var sorted: [ScanHistoryEntry] { entries.sorted { $0.timestamp < $1.timestamp } }
    private var latest: ScanHistoryEntry? { sorted.last }
    private var previous: ScanHistoryEntry? {
        guard sorted.count > 1 else { return nil }
        return sorted[sorted.count - 2]
    }
    private var reclaimableChange: Int {
        guard let latest = latest, let previous = previous else { return 0 }
        return latest.reclaimableBytes - previous.reclaimableBytes
    }
    private var findingChange: Int {
        guard let latest = latest, let previous = previous else { return 0 }
        return latest.findingCount - previous.findingCount
    }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Growth Analysis", icon: "chart.line.uptrend.xyaxis")
                if let latest = latest, previous != nil {
                    HStack(spacing: 24) {
                        ChangeBadge(
                            label: "Reclaimable change",
                            value: formatBytes(abs(reclaimableChange)),
                            isUp: reclaimableChange > 0,
                            color: reclaimableChange > 0 ? XTheme.warning : XTheme.safe
                        )
                        ChangeBadge(
                            label: "Finding change",
                            value: "\(abs(findingChange))",
                            isUp: findingChange > 0,
                            color: findingChange > 0 ? XTheme.warning : XTheme.safe
                        )
                        StatBadge(icon: "calendar", label: "Latest scan", value: latest.timestamp.formatted(date: .abbreviated, time: .shortened), color: XTheme.textSecondary)
                    }
                } else {
                    Text("Need at least two scans to show growth trends.")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)
                }
            }
        }
    }
}

private struct ChangeBadge: View {
    let label: String
    let value: String
    let isUp: Bool
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: isUp ? "arrow.up.forward" : "arrow.down.forward")
                    .font(.system(size: 12))
                    .foregroundStyle(color)
                Text(label)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textTertiary)
            }
            Text(value)
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(color)
        }
    }
}

private struct HistoryList: View {
    let entries: [ScanHistoryEntry]
    @EnvironmentObject var runner: XMacRunner
    @State private var showingClearConfirmation = false

    private var reversed: [ScanHistoryEntry] { entries.sorted { $0.timestamp > $1.timestamp } }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    XSectionHeader(title: "Recent Scans", icon: "list.bullet.rectangle", count: entries.count)
                    
                    Button("Clear History") {
                        showingClearConfirmation = true
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .tint(XTheme.danger)
                    .disabled(entries.isEmpty)
                    .confirmationDialog(
                        "Clear all scan history?",
                        isPresented: $showingClearConfirmation,
                        titleVisibility: .visible
                    ) {
                        Button("Clear All History", role: .destructive) {
                            runner.historyStore.clear()
                        }
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("This will permanently remove all \(entries.count) scan history entries. This action cannot be undone.")
                    }
                }
                
                LazyVStack(spacing: 8) {
                    ForEach(reversed) { entry in
                        HStack {
                            VStack(alignment: .leading, spacing: 3) {
                                Text(entry.timestamp.formatted(date: .abbreviated, time: .shortened))
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text("Mode: \(entry.mode)")
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                            Spacer()
                            VStack(alignment: .trailing, spacing: 3) {
                                Text(formatBytes(entry.reclaimableBytes))
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.safe)
                                Text("\(entry.findingCount) findings • \(String(format: "%.1f", entry.durationSeconds))s")
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                        }
                        .padding(.vertical, 6)
                        .contentShape(Rectangle())
                        .contextMenu {
                            Button("Re-run this scan") {
                                reRunScan(mode: entry.mode)
                            }
                            
                            Button("Export findings") {
                                exportEntry(entry)
                            }
                        }
                        Divider().background(XTheme.cardBorder)
                    }
                }
            }
        }
    }
    
    private func reRunScan(mode: String) {
        switch mode {
        case "clean":
            runner.startCleanScan()
        case "neural":
            runner.startNeuralScan()
        case "full":
            runner.startFullScan()
        case "maintain":
            runner.startMaintainScan()
        case "disk":
            runner.startDiskScan()
        default:
            runner.startFullScan()
        }
    }
    
    private func exportEntry(_ entry: ScanHistoryEntry) {
        let summary = """
        Scan Summary - \(entry.timestamp.formatted(date: .abbreviated, time: .shortened))
        Mode: \(entry.mode)
        Findings: \(entry.findingCount)
        Reclaimable: \(formatBytes(entry.reclaimableBytes))
        Duration: \(String(format: "%.1f", entry.durationSeconds))s
        """
        
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(summary, forType: .string)
    }
}

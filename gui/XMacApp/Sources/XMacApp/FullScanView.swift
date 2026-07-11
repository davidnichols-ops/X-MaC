import SwiftUI

struct FullScanView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if runner.isScanning {
                    ScanProgressView(phase: runner.scanPhase)
                } else if let err = runner.error {
                    ScanErrorView(message: err)
                } else if runner.findings.isEmpty {
                    EmptyScanView(message: "No findings. Run a scan to see results.")
                } else {
                    if let cleanupMessage = runner.cleanupMessage {
                        cleanupResultsBanner(message: cleanupMessage)
                    }
                    
                    if !runner.findings.isEmpty && !runner.isScanning {
                        FullScanCleanupToolbar()
                    }
                    
                    FullScanSummary(runner: runner)

                    FindingsGroupedList(findings: runner.findings)
                }
            }
            .padding(20)
        }
        .background(XTheme.bgPrimary)
    }
}

// MARK: - Summary

struct FullScanSummary: View {
    let runner: XMacRunner

    private var totalReclaimable: Int {
        runner.findings.compactMap { $0.size_bytes }.reduce(0, +)
    }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Scan Summary", icon: "chart.bar.doc.horizontal", count: runner.findings.count)

                HStack(spacing: 24) {
                    StatBadge(
                        icon: "doc.text.magnifyingglass",
                        label: "Findings",
                        value: "\(runner.findings.count)",
                        color: XTheme.accent
                    )
                    StatBadge(
                        icon: "externaldrive",
                        label: "Reclaimable",
                        value: formatBytes(totalReclaimable),
                        color: XTheme.safe
                    )
                    StatBadge(
                        icon: "clock",
                        label: "Duration",
                        value: String(format: "%.1fs", runner.scanDuration),
                        color: XTheme.info
                    )
                    StatBadge(
                        icon: "memorychip",
                        label: "Peak Memory",
                        value: String(format: "%.0f MB", runner.peakMemoryMB),
                        color: XTheme.medium
                    )
                }
            }
        }
    }
}

// MARK: - Grouped Findings List

struct FindingsGroupedList: View {
    let findings: [Finding]
    @EnvironmentObject var runner: XMacRunner

    private var grouped: [(String, [Finding])] {
        Dictionary(grouping: findings, by: { $0.category })
            .sorted { $0.key < $1.key }
    }

    var body: some View {
        LazyVStack(spacing: 16) {
            ForEach(grouped, id: \.0) { category, items in
                XCard {
                    VStack(alignment: .leading, spacing: 10) {
                        XSectionHeader(title: category.replacingOccurrences(of: "_", with: " ").capitalized, icon: "folder", count: items.count)

                        LazyVStack(spacing: 8) {
                            ForEach(items) { finding in
                                if finding.target.type == "Path" {
                                    SelectableFindingRow(finding: finding)
                                } else {
                                    FindingRow(finding: finding)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Finding Row

struct FindingRow: View {
    let finding: Finding

    private var isPathTarget: Bool {
        finding.target.type == "Path" && !finding.target.value.isEmpty
    }

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: finding.severityIcon)
                .font(.system(size: 16, weight: .medium))
                .foregroundStyle(finding.severityColor)
                .frame(width: 22, height: 22)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(finding.title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                        .lineLimit(1)

                    Spacer()

                    if !finding.sizeFormatted.isEmpty {
                        Text(finding.sizeFormatted)
                            .font(.system(size: 11, weight: .medium))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                }

                if isPathTarget {
                    FilePathDisplay(path: finding.target.value, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                }

                Text(finding.description)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(2)

                if let hint = finding.remediation_hint, !hint.isEmpty {
                    HStack(spacing: 4) {
                        Image(systemName: "lightbulb")
                            .font(.system(size: 9))
                            .foregroundStyle(XTheme.warning)
                        Text(hint)
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    .padding(.top, 2)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Shared Components

struct StatBadge: View {
    let icon: String
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: icon)
                    .font(.system(size: 12))
                    .foregroundStyle(color)
                Text(label)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textTertiary)
            }
            Text(value)
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(XTheme.textPrimary)
        }
    }
}

struct ScanProgressView: View {
    @EnvironmentObject var runner: XMacRunner
    let phase: String

    var body: some View {
        XCard {
            VStack(spacing: 14) {
                ProgressView(value: runner.scanProgress)
                    .tint(XTheme.accent)
                    .scaleEffect(y: 0.8)
                Text(phase.isEmpty ? "Scanning..." : phase)
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
                Text("\(Int(runner.scanProgress * 100))%")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textTertiary)
                Button("Stop Scan") {
                    runner.stopScan()
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .tint(XTheme.danger)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
        }
    }
}

struct ScanErrorView: View {
    @EnvironmentObject var runner: XMacRunner
    let message: String

    var body: some View {
        XCard {
            HStack(spacing: 12) {
                Image(systemName: "xmark.octagon.fill")
                    .font(.system(size: 22))
                    .foregroundStyle(XTheme.critical)
                VStack(alignment: .leading, spacing: 4) {
                    Text("Scan Error")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                    Text(message)
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)
                }
                Spacer()
                Button("Dismiss") {
                    runner.error = nil
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
            }
        }
    }
}

struct EmptyScanView: View {
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "tray")
                .font(.system(size: 36, weight: .light))
                .foregroundStyle(XTheme.textTertiary)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 60)
    }
}

// MARK: - Cleanup Toolbar

struct FullScanCleanupToolbar: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var showingTrashAlert = false

    var body: some View {
        XCard {
            VStack(spacing: 12) {
                HStack(spacing: 12) {
                    Button("Select safe items") {
                        runner.selectAllSafeFindings()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    
                    if !runner.selectedPaths.isEmpty {
                        Button("Clear") {
                            runner.clearSelection()
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                    }
                    
                    Spacer()
                    
                    if !runner.selectedPaths.isEmpty {
                        Text("\(runner.selectedPaths.count) selected")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundStyle(XTheme.textPrimary)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(XTheme.accent.opacity(0.2))
                            .clipShape(Capsule())
                        
                        Button {
                            showingTrashAlert = true
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: "trash")
                                Text("Move to Trash")
                            }
                        }
                        .buttonStyle(.borderedProminent)
                        .controlSize(.small)
                        .tint(XTheme.danger)
                        .disabled(runner.selectedPaths.isEmpty)
                    }
                }
            }
        }
        .alert("Confirm Move to Trash", isPresented: $showingTrashAlert) {
            Button("Cancel", role: .cancel) { }
            Button("Move to Trash", role: .destructive) {
                runner.trashSelected()
            }
        } message: {
            Text("Move \(runner.selectedPaths.count) selected items to Trash? This action can be undone from the Trash.")
        }
    }
}

// MARK: - Cleanup Results Banner

func cleanupResultsBanner(message: String) -> some View {
    XCard {
        HStack(spacing: 8) {
            Image(systemName: "checkmark.circle.fill")
                .font(.system(size: 16))
                .foregroundStyle(XTheme.safe)
            Text(message)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(XTheme.textPrimary)
            Spacer()
        }
    }
}

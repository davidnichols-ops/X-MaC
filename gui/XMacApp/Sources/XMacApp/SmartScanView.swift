import SwiftUI

struct SmartScanView: View {
    @EnvironmentObject var runner: XMacRunner

    private var cleanFindings: [Finding] {
        runner.findings.filter { $0.engine == "clean" }
    }

    private var gnnOnlyPaths: Set<String> {
        let rulePaths = Set(cleanFindings.map { $0.target.value })
        return Set(runner.gnnScores.filter { $0.safety_score >= 0.7 }.map { $0.path })
            .subtracting(rulePaths)
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if runner.isScanning {
                    ScanProgressView(phase: runner.scanPhase)
                } else if let err = runner.error {
                    ScanErrorView(message: err)
                } else if cleanFindings.isEmpty && runner.gnnScores.isEmpty {
                    EmptyScanView(message: "No data available. Run a scan to compare results.")
                } else {
                    SmartComparisonHeader(
                        ruleCount: cleanFindings.count,
                        gnnCount: runner.gnnScores.count,
                        gnnOnlyCount: gnnOnlyPaths.count
                    )

                    HStack(alignment: .top, spacing: 16) {
                        SmartRuleColumn(findings: cleanFindings)
                            .frame(maxWidth: .infinity)

                        SmartGNNColumn(scores: runner.gnnScores)
                            .frame(maxWidth: .infinity)
                    }

                    if !gnnOnlyPaths.isEmpty {
                        GNNDifferenceCard(paths: gnnOnlyPaths, scores: runner.gnnScores)
                    }
                }
            }
            .padding(20)
        }
        .background(XTheme.bgPrimary)
    }
}

// MARK: - Header

struct SmartComparisonHeader: View {
    let ruleCount: Int
    let gnnCount: Int
    let gnnOnlyCount: Int

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Rule-Based vs GNN", icon: "arrow.left.arrow.right")

                HStack(spacing: 24) {
                    StatBadge(
                        icon: "ruler",
                        label: "Rule Findings",
                        value: "\(ruleCount)",
                        color: XTheme.accent
                    )
                    StatBadge(
                        icon: "brain.head.profile",
                        label: "GNN Scores",
                        value: "\(gnnCount)",
                        color: XTheme.anomaly
                    )
                    StatBadge(
                        icon: "plus.circle",
                        label: "GNN-Only Detections",
                        value: "\(gnnOnlyCount)",
                        color: XTheme.warning
                    )
                }
            }
        }
    }
}

// MARK: - Rule Column

struct SmartRuleColumn: View {
    let findings: [Finding]

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Rule-Based", icon: "ruler", count: findings.count)

                if findings.isEmpty {
                    Text("No rule-based findings.")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 12)
                } else {
                    LazyVStack(spacing: 8) {
                        ForEach(findings.prefix(50)) { finding in
                            FindingRow(finding: finding)
                        }
                    }
                }
            }
        }
    }
}

// MARK: - GNN Column

struct SmartGNNColumn: View {
    let scores: [GNNScore]
    @EnvironmentObject var runner: XMacRunner
    @State private var showingTrashAlert = false

    private var sortedScores: [GNNScore] {
        scores.sorted { $0.safety_score < $1.safety_score }
    }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "GNN-Scored", icon: "brain.head.profile", count: scores.count)

                if scores.isEmpty {
                    Text("No GNN scores available.")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 12)
                } else {
                    LazyVStack(spacing: 6) {
                        ForEach(sortedScores.prefix(50)) { score in
                            CompactGNNRow(score: score)
                        }
                    }
                    
                    if !runner.selectedGNNPaths.isEmpty {
                        Divider().background(XTheme.cardBorder)
                        GNNCleanupBar(showingTrashAlert: $showingTrashAlert)
                    }
                }
            }
        }
        .alert("Confirm Move to Trash", isPresented: $showingTrashAlert) {
            Button("Cancel", role: .cancel) { }
            Button("Move to Trash", role: .destructive) {
                runner.trashSelectedGNNPaths()
            }
        } message: {
            Text("Move \(runner.selectedGNNPaths.count) GNN-recommended items to Trash? This action can be undone from the Trash.")
        }
    }
}

struct CompactGNNRow: View {
    let score: GNNScore
    @EnvironmentObject var runner: XMacRunner
    
    private var isSelected: Bool {
        runner.selectedGNNPaths.contains(score.path)
    }

    var body: some View {
        Button {
            runner.toggleGNNSelection(path: score.path)
        } label: {
            HStack(spacing: 8) {
                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 12))
                    .foregroundStyle(isSelected ? XTheme.safe : XTheme.textTertiary)
                    .frame(width: 16)
                
                Image(systemName: score.safetyIcon)
                    .font(.system(size: 11))
                    .foregroundStyle(score.safetyColor)
                    .frame(width: 16)

                Text(score.path)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                    .lineLimit(1)
                    .truncationMode(.middle)

                Spacer()

                Text(String(format: "%.0f%%", score.safety_score * 100))
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(score.safetyColor)
                    .frame(width: 36, alignment: .trailing)
            }
            .padding(.vertical, 2)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(runner.isScanning)
        .help("Select for review and move to Trash")
    }
}

// MARK: - GNN Cleanup Bar

struct GNNCleanupBar: View {
    @EnvironmentObject var runner: XMacRunner
    @Binding var showingTrashAlert: Bool

    var body: some View {
        HStack(spacing: 12) {
            Text("\(runner.selectedGNNPaths.count) selected")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(XTheme.textPrimary)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(XTheme.accent.opacity(0.2))
                .clipShape(Capsule())
            
            Spacer()
            
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
            .disabled(runner.selectedGNNPaths.isEmpty)
        }
        .padding(.top, 8)
    }
}

// MARK: - GNN Difference Card

struct GNNDifferenceCard: View {
    let paths: Set<String>
    let scores: [GNNScore]
    @EnvironmentObject var runner: XMacRunner

    private var gnnOnlyScores: [GNNScore] {
        scores.filter { paths.contains($0.path) }
            .sorted { ($0.size_bytes ?? 0) > ($1.size_bytes ?? 0) }
    }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                HStack {
                    XSectionHeader(title: "GNN Found (Rules Missed)", icon: "lightbulb", count: gnnOnlyScores.count)
                    
                    Button("Select All GNN-Only") {
                        let gnnOnlyPaths = gnnOnlyScores.map(\.path)
                        runner.selectGNNPaths(gnnOnlyPaths)
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(gnnOnlyScores.isEmpty)
                }

                Text("Files flagged as safe by the GNN but not detected by rule-based scanning.")
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textTertiary)

                LazyVStack(spacing: 6) {
                    ForEach(gnnOnlyScores.prefix(30)) { score in
                        Button {
                            runner.toggleGNNSelection(path: score.path)
                        } label: {
                            HStack(spacing: 8) {
                                Image(systemName: runner.selectedGNNPaths.contains(score.path) ? "checkmark.square.fill" : "square")
                                    .font(.system(size: 12))
                                    .foregroundStyle(runner.selectedGNNPaths.contains(score.path) ? XTheme.safe : XTheme.textTertiary)
                                    .frame(width: 16)
                                
                                Image(systemName: "plus.circle.fill")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.warning)

                                Text(score.path)
                                    .font(.system(size: 11, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)

                                Spacer()

                                if !score.sizeFormatted.isEmpty {
                                    Text(score.sizeFormatted)
                                        .font(.system(size: 10, weight: .medium))
                                        .foregroundStyle(XTheme.textSecondary)
                                }

                                Text(String(format: "%.0f%%", score.safety_score * 100))
                                    .font(.system(size: 10, weight: .semibold))
                                    .foregroundStyle(score.safetyColor)
                                    .frame(width: 36, alignment: .trailing)
                            }
                            .padding(.vertical, 2)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .disabled(runner.isScanning)
                        .help("Select for review and move to Trash")
                    }
                }
            }
        }
    }
}

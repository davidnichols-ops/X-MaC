import SwiftUI

struct NeuralScanView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        Group {
            if runner.isScanning {
                NeuralScanningView(phase: runner.scanPhase, nodeCount: runner.graphNodeCount)
            } else if let err = runner.error {
                ScanErrorView(message: err)
            } else if let response = runner.gnnResponse {
                NeuralResultsView(response: response, runner: runner)
            } else {
                NeuralIdleView()
            }
        }
        .background(XTheme.bgPrimary)
    }
}

// MARK: - Scanning State

struct NeuralScanningView: View {
    @EnvironmentObject var runner: XMacRunner
    let phase: String
    let nodeCount: Int

    @State private var animateGlow = false

    var body: some View {
        VStack(spacing: 28) {
            Spacer()

            ZStack {
                Circle()
                    .stroke(XTheme.cardBorder, lineWidth: 2)
                    .frame(width: 120, height: 120)

                Circle()
                    .trim(from: 0, to: 0.3)
                    .stroke(XTheme.anomaly, style: StrokeStyle(lineWidth: 3, lineCap: .round))
                    .frame(width: 120, height: 120)
                    .rotationEffect(.degrees(animateGlow ? 360 : 0))
                    .animation(.linear(duration: 2).repeatForever(autoreverses: false), value: animateGlow)

                Image(systemName: "network")
                    .font(.system(size: 40, weight: .light))
                    .foregroundStyle(XTheme.anomaly)
                    .xGlow(XTheme.anomaly, radius: 16)
            }
            .onAppear { animateGlow = true }

            VStack(spacing: 8) {
                Text(phase.isEmpty ? "Running neural scan..." : phase)
                    .font(.system(size: 15, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)

                if nodeCount > 0 {
                    Text("\(nodeCount) graph nodes extracted")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)
                }

                Text("Detecting cross-directory anomaly clusters that rule-based scans miss.")
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textTertiary)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: 320)
            }

            VStack(spacing: 6) {
                ProgressView(value: runner.scanProgress)
                    .tint(XTheme.anomaly)
                    .scaleEffect(y: 0.8)
                    .frame(maxWidth: 260)
                Text("\(Int(runner.scanProgress * 100))%")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textTertiary)
            }

            Button("Stop Neural Scan") {
                runner.stopScan()
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
            .tint(XTheme.danger)

            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Idle State

struct NeuralIdleView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "brain.head.profile")
                .font(.system(size: 56, weight: .light))
                .foregroundStyle(XTheme.anomaly)
                .xGlow(XTheme.anomaly, radius: 14)

            VStack(spacing: 8) {
                Text("Neural Scan")
                    .font(.system(size: 24, weight: .bold))
                    .foregroundStyle(XTheme.textPrimary)

                Text("GNN-powered analysis of your file system")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
            }

            XCard {
                VStack(alignment: .leading, spacing: 10) {
                    NeuralFeatureRow(icon: "network", text: "Extracts a file system graph (nodes + edges)")
                    NeuralFeatureRow(icon: "cpu", text: "Runs on-device CoreML GNN inference")
                    NeuralFeatureRow(icon: "shield.checkered", text: "Scores each file for safety & anomaly")
                    NeuralFeatureRow(icon: "trash", text: "Generates an impact-weighted purge plan")
                    Divider().background(XTheme.cardBorder)
                    Text("Irreplaceable value:")
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                    Text("Rule-based cleaners miss bloat that is spread across many directories. The GNN sees the whole graph at once and surfaces anomaly hotspots, duplicate-like patterns, and oversized clusters that only become obvious when files are connected by location, size, and age.")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                }
            }
            .frame(maxWidth: 420)

            Button(action: { runner.startNeuralScan() }) {
                HStack(spacing: 8) {
                    Image(systemName: "play.fill")
                        .font(.system(size: 12, weight: .semibold))
                    Text("Start Neural Scan")
                        .font(.system(size: 13, weight: .semibold))
                }
                .foregroundStyle(XTheme.bgPrimary)
                .padding(.horizontal, 20)
                .padding(.vertical, 10)
                .background(XTheme.anomaly)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .xGlow(XTheme.anomaly)
            }
            .buttonStyle(.plain)

            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct NeuralFeatureRow: View {
    let icon: String
    let text: String

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 12))
                .foregroundStyle(XTheme.anomaly)
                .frame(width: 20)
            Text(text)
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textSecondary)
            Spacer()
        }
    }
}

// MARK: - Results View

struct NeuralResultsView: View {
    let response: GNNResponse
    let runner: XMacRunner
    @State private var selectedTab = 0

    var body: some View {
        VStack(spacing: 0) {
            NeuralResultsSummary(summary: response.summary, scores: response.scores, runner: runner)

            TabView(selection: $selectedTab) {
                NeuralScoresTab(scores: response.scores)
                    .tabItem {
                        Label("Scores", systemImage: "chart.bar")
                    }
                    .tag(0)

                NeuralPurgeTab(purgePlan: response.purge_plan)
                    .tabItem {
                        Label("Smart Purge", systemImage: "trash")
                    }
                    .tag(1)
            }
        }
        .background(XTheme.bgPrimary)
    }
}

// MARK: - Results Summary

struct NeuralResultsSummary: View {
    let summary: GNNSummary?
    let scores: [GNNScore]
    let runner: XMacRunner

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack {
                    XSectionHeader(title: "Neural Analysis Summary", icon: "brain.head.profile", count: scores.count)
                    Spacer()
                    if let plan = runner.purgePlan, plan.cleanup_confidence == "FALLBACK" {
                        HStack(spacing: 4) {
                            Image(systemName: "exclamationmark.triangle")
                                .font(.system(size: 10))
                            Text("Rule-based fallback")
                                .font(.system(size: 10, weight: .semibold))
                        }
                        .foregroundStyle(XTheme.warning)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(XTheme.warning.opacity(0.12))
                        .clipShape(Capsule())
                    }
                }

                if let s = summary {
                    HStack(spacing: 20) {
                        NeuralStatBadge(label: "Total Files", value: "\(s.total_files)", color: XTheme.accent)
                        NeuralStatBadge(label: "Safe", value: "\(s.safe_files)", color: XTheme.safe)
                        NeuralStatBadge(label: "Review", value: "\(s.review_files)", color: XTheme.warning)
                        NeuralStatBadge(label: "Danger", value: "\(s.danger_files)", color: XTheme.danger)
                    }

                    HStack(spacing: 20) {
                        NeuralStatBadge(
                            label: "Avg Safety",
                            value: String(format: "%.0f%%", s.avg_safety * 100),
                            color: XTheme.safe
                        )
                        NeuralStatBadge(
                            label: "Avg Anomaly",
                            value: String(format: "%.0f%%", s.avg_anomaly * 100),
                            color: XTheme.anomaly
                        )
                        if let reclaim = s.potential_reclaim_bytes, reclaim > 0 {
                            NeuralStatBadge(
                                label: "Potential Reclaim",
                                value: formatBytes(reclaim),
                                color: XTheme.accent
                            )
                        }
                        NeuralStatBadge(
                            label: "Duration",
                            value: String(format: "%.1fs", runner.scanDuration),
                            color: XTheme.info
                        )
                    }
                }
            }
        }
        .padding(20)
    }
}

struct NeuralStatBadge: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textTertiary)
            Text(value)
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(color)
        }
    }
}

// MARK: - Scores Tab

struct NeuralScoresTab: View {
    let scores: [GNNScore]

    private var sortedScores: [GNNScore] {
        scores.sorted { $0.safety_score < $1.safety_score }
    }

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                ForEach(sortedScores) { score in
                    NeuralScoreRow(score: score)
                }
            }
            .padding(20)
        }
    }
}

struct NeuralScoreRow: View {
    let score: GNNScore

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Image(systemName: score.safetyIcon)
                        .font(.system(size: 14))
                        .foregroundStyle(score.safetyColor)
                        .accessibilityLabel("Safety: \(String(format: "%.0f", score.safety_score * 100)) percent")

                    FilePathDisplay(path: score.path, fontSize: 12, pathFontSize: 10, showIcon: false)

                    Spacer()

                    Text(score.label)
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 2)
                        .background(XTheme.bgTertiary)
                        .clipShape(Capsule())

                    if !score.sizeFormatted.isEmpty {
                        Text(score.sizeFormatted)
                            .font(.system(size: 11, weight: .medium))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                }

                HStack(spacing: 16) {
                    NeuralScoreBar(label: "Safety", value: score.safety_score, color: XTheme.safe)
                    NeuralScoreBar(label: "Anomaly", value: score.anomaly_score, color: XTheme.anomaly)
                    NeuralScoreBar(label: "Confidence", value: score.confidence ?? 0, color: XTheme.info)
                }

                Text(score.explanationText)
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(2)
            }
        }
    }
}

struct NeuralScoreBar: View {
    let label: String
    let value: Double
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack {
                Text(label)
                    .font(.system(size: 9))
                    .foregroundStyle(XTheme.textTertiary)
                Spacer()
                Text(String(format: "%.0f%%", value * 100))
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(color)
            }
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 3)
                        .fill(XTheme.bgTertiary)
                        .frame(height: 5)
                    RoundedRectangle(cornerRadius: 3)
                        .fill(color)
                        .frame(width: geo.size.width * CGFloat(value), height: 5)
                }
            }
            .frame(height: 5)
        }
    }
}

// MARK: - Smart Purge Tab

struct NeuralPurgeTab: View {
    @EnvironmentObject var runner: XMacRunner
    let purgePlan: PurgePlan?

    private var selectedReclaimable: Int {
        runner.gnnScores
            .filter { runner.selectedGNNPaths.contains($0.path) }
            .compactMap { $0.size_bytes }
            .reduce(0, +)
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if let plan = purgePlan {
                    PurgeConfidenceCard(confidence: plan.cleanup_confidence)

                    GNNCleanupToolbar(
                        selectedCount: runner.selectedGNNPaths.count,
                        selectedSize: selectedReclaimable,
                        safeCount: plan.impact_weighted_order.filter { $0.safety_score >= 0.7 }.count
                    )

                    PurgeOrderList(items: plan.impact_weighted_order)

                    AnomalyHotspotsList(hotspots: plan.anomaly_hotspots)

                    CrossDirPatternsList(patterns: plan.cross_directory_patterns)
                } else {
                    EmptyScanView(message: "No purge plan available.")
                }
            }
            .padding(20)
        }
    }
}

struct GNNCleanupToolbar: View {
    @EnvironmentObject var runner: XMacRunner
    let selectedCount: Int
    let selectedSize: Int
    let safeCount: Int
    @State private var showingConfirmation = false

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 10) {
                    Image(systemName: "trash.circle")
                        .font(.system(size: 18))
                        .foregroundStyle(XTheme.safe)
                    VStack(alignment: .leading, spacing: 3) {
                        Text("GNN Cleanup")
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                        Text("Selected: \(formatBytes(selectedSize)) from \(selectedCount) files")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    Spacer()
                }

                HStack(spacing: 10) {
                    Button("Select all safe") {
                        runner.selectAllGNNSafePaths()
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(safeCount == 0 || runner.isCleaning)

                    Button("Clear") {
                        runner.clearGNNSelection()
                    }
                    .buttonStyle(.borderless)
                    .controlSize(.small)
                    .disabled(selectedCount == 0 || runner.isCleaning)

                    Spacer()

                    Button {
                        showingConfirmation = true
                    } label: {
                        Label(runner.isCleaning ? "Moving…" : "Move to Trash", systemImage: "trash")
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(XTheme.safe)
                    .controlSize(.small)
                    .disabled(selectedCount == 0 || runner.isCleaning)
                    .confirmationDialog("Move GNN-selected items to Trash?", isPresented: $showingConfirmation, titleVisibility: .visible) {
                        Button("Move \(selectedCount) items to Trash", role: .destructive) {
                            runner.trashSelectedGNNPaths()
                        }
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("These paths were recommended by the neural model. They will be moved to macOS Trash, not permanently deleted.")
                    }
                }

                if let message = runner.cleanupMessage {
                    Text(message)
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(XTheme.safe)
                }
            }
        }
    }
}

// MARK: - Confidence Card

struct PurgeConfidenceCard: View {
    let confidence: String

    private var confidenceColor: Color {
        switch confidence {
        case "VERY HIGH": return XTheme.safe
        case "HIGH": return XTheme.low
        case "MODERATE": return XTheme.warning
        case "LOW": return XTheme.danger
        default: return XTheme.info
        }
    }

    var body: some View {
        XCard {
            HStack(spacing: 12) {
                Image(systemName: "shield.checkered")
                    .font(.system(size: 20))
                    .foregroundStyle(confidenceColor)
                    .xGlow(confidenceColor)

                VStack(alignment: .leading, spacing: 4) {
                    Text("Cleanup Confidence")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                    Text(confidence)
                        .font(.system(size: 18, weight: .bold))
                        .foregroundStyle(confidenceColor)
                }

                Spacer()
            }
        }
    }
}

// MARK: - Purge Order List

struct PurgeOrderList: View {
    let items: [PurgeItem]

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Impact-Weighted Deletion Order", icon: "arrow.down.circle", count: items.count)

                if items.isEmpty {
                    Text("No items recommended for deletion.")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 8)
                } else {
                    LazyVStack(spacing: 6) {
                        ForEach(Array(items.enumerated()), id: \.element.id) { idx, item in
                            PurgeItemRow(item: item, rank: idx + 1)
                        }
                    }
                }
            }
        }
    }
}

struct PurgeItemRow: View {
    @EnvironmentObject var runner: XMacRunner
    let item: PurgeItem
    let rank: Int

    private var isSelected: Bool { runner.selectedGNNPaths.contains(item.path) }
    private var isSafe: Bool { item.safety_score >= 0.7 }
    private var isActionable: Bool { runner.isSafeCleanupPath(item.path) }

    var body: some View {
        Button {
            if isActionable {
                runner.toggleGNNSelection(path: item.path)
            }
        } label: {
            HStack(spacing: 10) {
                Text("\(rank)")
                    .font(.system(size: 11, weight: .bold))
                    .foregroundStyle(XTheme.textTertiary)
                    .frame(width: 22)

                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 14))
                    .foregroundStyle(isSelected ? XTheme.safe : XTheme.textTertiary)
                    .frame(width: 20)

                Image(systemName: isSafe ? "checkmark.circle.fill" : "questionmark.circle.fill")
                    .font(.system(size: 12))
                    .foregroundStyle(isSafe ? XTheme.safe : XTheme.warning)

                VStack(alignment: .leading, spacing: 2) {
                    FilePathDisplay(path: item.path, fontSize: 12, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                    Text("Safety: \(String(format: "%.0f%%", item.safety_score * 100))  |  Impact: \(formatBytes(Int(item.impact_score)))")
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textTertiary)
                }

                Spacer()

                Text(item.sizeFormatted)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.accent)
            }
            .padding(.vertical, 4)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!isActionable || runner.isCleaning)
        .opacity(isActionable ? 1.0 : 0.5)
        .help(isActionable ? "Tap to select for cleanup" : "Protected path")
    }
}

// MARK: - Anomaly Hotspots

struct AnomalyHotspotsList: View {
    let hotspots: [AnomalyHotspot]

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Anomaly Hotspots", icon: "flame", count: hotspots.count)

                if hotspots.isEmpty {
                    Text("No anomaly hotspots detected.")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 8)
                } else {
                    LazyVStack(spacing: 8) {
                        ForEach(hotspots) { hotspot in
                            AnomalyHotspotRow(hotspot: hotspot)
                        }
                    }
                }
            }
        }
    }
}

struct AnomalyHotspotRow: View {
    let hotspot: AnomalyHotspot

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: "flame.fill")
                .font(.system(size: 12))
                .foregroundStyle(XTheme.anomaly)

            FilePathDisplay(path: hotspot.directory, fontSize: 12, pathFontSize: 9, showIcon: false, maxPathLines: 1)

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text(String(format: "%.0f%%", hotspot.avg_anomaly * 100))
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(XTheme.anomaly)
                Text("\(hotspot.anomaly_count) anomalous files")
                    .font(.system(size: 9))
                    .foregroundStyle(XTheme.textTertiary)
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Cross-Directory Patterns

struct CrossDirPatternsList: View {
    let patterns: [CrossDirPattern]

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Cross-Directory Patterns", icon: "square.grid.3x3", count: patterns.count)

                if patterns.isEmpty {
                    Text("No cross-directory patterns found.")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 8)
                } else {
                    LazyVStack(spacing: 10) {
                        ForEach(patterns) { pattern in
                            CrossDirPatternRow(pattern: pattern)
                        }
                    }
                }
            }
        }
    }
}

struct CrossDirPatternRow: View {
    @EnvironmentObject var runner: XMacRunner
    let pattern: CrossDirPattern

    private var selectedExampleCount: Int {
        pattern.example_paths.filter { runner.selectedGNNPaths.contains($0) }.count
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Image(systemName: "doc.on.doc")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.info)

                Text(pattern.pattern.replacingOccurrences(of: "_", with: " ").capitalized)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)

                Spacer()

                Text("\(pattern.file_count) files")
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textTertiary)

                Text(pattern.sizeFormatted)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.accent)
            }

            if !pattern.example_paths.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Examples:")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(XTheme.textSecondary)
                    ForEach(pattern.example_paths.prefix(3), id: \.self) { path in
                        HStack(spacing: 4) {
                            Image(systemName: runner.selectedGNNPaths.contains(path) ? "checkmark.square.fill" : "square")
                                .font(.system(size: 10))
                                .foregroundStyle(runner.selectedGNNPaths.contains(path) ? XTheme.safe : XTheme.textTertiary)
                            FilePathDisplay(path: path, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                        }
                    }
                }
            }

            HStack {
                Spacer()
                Button(selectedExampleCount > 0 ? "Selected \(selectedExampleCount)" : "Select examples") {
                    runner.selectGNNPattern(examplePaths: pattern.example_paths)
                }
                .buttonStyle(.bordered)
                .controlSize(.mini)
                .disabled(pattern.example_paths.isEmpty || runner.isCleaning)
            }
        }
        .padding(.vertical, 4)
    }
}

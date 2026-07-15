import SwiftUI

// MARK: - MemoryStats model (parsed from xmac ram-boost JSON findings)

struct MemoryStatsSnapshot: Codable, Hashable {
    let total_bytes: UInt64
    let used_bytes: UInt64
    let available_bytes: UInt64
    let app_memory_bytes: UInt64
    let wired_bytes: UInt64
    let compressed_bytes: UInt64
    let swap_used_bytes: UInt64
    let swap_total_bytes: UInt64
    let memory_pressure: String
    let top_consumers: [ProcessMemoryEntry]
}

struct ProcessMemoryEntry: Codable, Hashable, Identifiable {
    let pid: UInt32
    let name: String
    let rss_bytes: UInt64
    let percent: Double
    var id: UInt32 { pid }
}

// MARK: - RamBoostView (Reworked with GNN integration)

struct RamBoostView: View {
    @EnvironmentObject var runner: XMacRunner

    // Kill confirmation state
    @State private var pendingKill: (pid: Int, name: String, force: Bool)? = nil

    // Boost state
    @State private var beforeStats: MemoryStatsSnapshot? = nil
    @State private var afterStats: MemoryStatsSnapshot? = nil
    @State private var freedBytes: UInt64 = 0
    @State private var freedSwapBytes: UInt64 = 0
    @State private var processesKilled: Int = 0
    @State private var isBoosting = false
    @State private var boostError: String? = nil
    @State private var hasRunBoost = false
    @State private var purgeSuccess: Bool = false
    @State private var purgeMessage: String = ""

    // Kill options
    @State private var killTopN: Int = 0
    @State private var killByName: String = ""
    @State private var minRssMB: Int = 500
    @State private var protectSystem: Bool = true
    @State private var forceKill: Bool = false
    @State private var purgeEnabled: Bool = true

    // GNN filter
    @State private var showOnlyActionable: Bool = false
    @State private var selectedActionFilter: String? = nil

    // Refresh timer
    @State private var autoRefresh: Bool = false
    @State private var refreshTimer: Timer?

    var body: some View {
        ScrollView {
            VStack(spacing: 18) {
                if runner.isOptimizing {
                    optimizeProgressView
                } else if let err = boostError {
                    ScanErrorView(message: err)
                } else if let snapshot = runner.optimizeResult {
                    // GNN system pressure prediction banner
                    if let gnn = snapshot.gnnResult {
                        gnnPressureBanner(gnn: gnn)
                    }

                    // Memory dashboard
                    memoryDashboardCard(snapshot: snapshot)

                    // GNN predictions (actionable items)
                    if let gnn = snapshot.gnnResult {
                        gnnPredictionsCard(snapshot: snapshot, gnn: gnn)
                    }

                    // Top processes detail
                    processListCard(snapshot: snapshot)

                    // Boost controls (purge + kill)
                    boostControlsCard

                    // After-boost results
                    if hasRunBoost, let before = beforeStats, let after = afterStats {
                        boostResultCard(before: before, after: after)
                    }

                    // Purge status
                    if hasRunBoost {
                        purgeStatusCard
                    }
                } else {
                    emptyStateCard
                }
            }
            .padding(24)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.optimizeResult == nil && !runner.isOptimizing {
                Task { await runner.runOptimize(topN: 15) }
            }
        }
        .onDisappear {
            refreshTimer?.invalidate()
            refreshTimer = nil
        }
        .onChange(of: autoRefresh) { enabled in
            if enabled {
                refreshTimer = Timer.scheduledTimer(withTimeInterval: 10, repeats: true) { _ in
                    Task { await runner.runOptimize(topN: 15) }
                }
            } else {
                refreshTimer?.invalidate()
                refreshTimer = nil
            }
        }
    }

    // MARK: - Empty State

    private var emptyStateCard: some View {
        XCard {
            VStack(spacing: 16) {
                Image(systemName: "memorychip")
                    .font(.system(size: 48, weight: .light))
                    .foregroundStyle(XTheme.metallicGradient)
                    .xGlow(XTheme.accent, radius: 8)

                Text("RAM Optimizer")
                    .font(.system(size: 20, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)

                Text("AI-powered memory analysis with GNN predictions. Collects telemetry, predicts pressure, and recommends per-process actions.")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
                    .multilineTextAlignment(.center)

                Button {
                    Task { await runner.runOptimize(topN: 15) }
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "brain.head.profile.fill")
                            .font(.system(size: 12))
                        Text("Analyze Memory")
                            .font(.system(size: 13, weight: .semibold))
                    }
                    .foregroundStyle(.white)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 8)
                    .background(XTheme.neuralGradient)
                    .clipShape(Capsule())
                }
                .buttonStyle(.plain)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
        }
    }

    // MARK: - Optimize Progress

    private var optimizeProgressView: some View {
        XCard {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.2)
                    .tint(XTheme.accent)

                Text("Analyzing Memory...")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)

                Text("Collecting telemetry, building graph, running GNN inference")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 30)
        }
    }

    // MARK: - GNN Pressure Banner

    private func gnnPressureBanner(gnn: MemoryGNNManager.OptimizationResult) -> some View {
        let pressure = gnn.systemPressure
        let color = pressureColor(pressure.level)
        let icon = pressureIcon(pressure.level)

        return XCard {
            HStack(spacing: 14) {
                Image(systemName: icon)
                    .font(.system(size: 28))
                    .foregroundStyle(color)
                    .xGlow(color, radius: 6)

                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 6) {
                        Text("GNN Prediction")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(XTheme.textTertiary)
                        if gnn.usingFallback {
                            Text("HEURISTIC")
                                .font(.system(size: 9, weight: .bold))
                                .foregroundStyle(XTheme.medium)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(XTheme.medium.opacity(0.2))
                                .clipShape(Capsule())
                        } else {
                            Text("NEURAL")
                                .font(.system(size: 9, weight: .bold))
                                .foregroundStyle(XTheme.accent)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(XTheme.accent.opacity(0.2))
                                .clipShape(Capsule())
                        }
                    }
                    Text("System pressure in 60s: \(pressure.level.capitalized)")
                        .font(.system(size: 15, weight: .bold))
                        .foregroundStyle(color)
                    Text("Confidence: \(Int(pressure.confidence * 100))% • \(gnn.predictions.count) processes analyzed")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                }

                Spacer()

                Button {
                    Task { await runner.runOptimize(topN: 15) }
                } label: {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 14))
                        .foregroundStyle(XTheme.textSecondary)
                }
                .buttonStyle(.plain)
            }
        }
    }

    // MARK: - Memory Dashboard

    private func memoryDashboardCard(snapshot: OptimizeSnapshot) -> some View {
        let pressureColor = pressureColor(snapshot.pressureLabel)
        let utilPct = snapshot.utilizationPct

        return XCard {
            VStack(alignment: .leading, spacing: 14) {
                // Header with auto-refresh toggle
                HStack {
                    XSectionHeader(title: "Memory Dashboard", icon: "gauge.with.dots.needle.67percent", count: nil)
                    Spacer()
                    Toggle(isOn: $autoRefresh) {
                        Text("Auto")
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .tint(XTheme.accent)
                    .labelsHidden()
                    .scaleEffect(0.8)
                }

                HStack(spacing: 20) {
                    // Circular gauge
                    ZStack {
                        Circle()
                            .stroke(XTheme.bgTertiary, lineWidth: 10)
                            .frame(width: 90, height: 90)
                        Circle()
                            .trim(from: 0, to: utilPct / 100.0)
                            .stroke(pressureColor, style: StrokeStyle(lineWidth: 10, lineCap: .round))
                            .rotationEffect(.degrees(-90))
                            .frame(width: 90, height: 90)
                            .xGlow(pressureColor, radius: 4)
                        VStack(spacing: 2) {
                            Text(String(format: "%.0f%%", utilPct))
                                .font(.system(size: 20, weight: .bold, design: .rounded))
                                .foregroundStyle(pressureColor)
                            Text("Used")
                                .font(.system(size: 9))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                    }

                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 6) {
                            Circle()
                                .fill(pressureColor)
                                .frame(width: 8, height: 8)
                                .xGlow(pressureColor, radius: 3)
                            Text(snapshot.pressureLabel.capitalized)
                                .font(.system(size: 14, weight: .semibold))
                                .foregroundStyle(pressureColor)
                        }
                        statRow(label: "Free", value: formatMB(snapshot.freeMB))
                        statRow(label: "Compressed", value: formatMB(snapshot.compressedMB))
                        statRow(label: "Swap", value: formatMB(snapshot.swapMB))
                        statRow(label: "Graph", value: "\(snapshot.nodeCount) nodes, \(snapshot.edgeCount) edges")
                    }
                    Spacer()
                }
            }
        }
    }

    // MARK: - GNN Predictions Card (Actionable Items)

    private func gnnPredictionsCard(snapshot: OptimizeSnapshot, gnn: MemoryGNNManager.OptimizationResult) -> some View {
        let predictions = gnn.predictions
        let actionable = predictions.filter { $0.action != "no_action" }
        let filtered = showOnlyActionable ? actionable : predictions

        return XCard {
            VStack(alignment: .leading, spacing: 12) {
                // Header
                HStack {
                    XSectionHeader(title: "AI Recommendations", icon: "brain.head.profile.fill", count: actionable.count > 0 ? actionable.count : nil)
                    Spacer()
                    if !actionable.isEmpty {
                        Toggle(isOn: $showOnlyActionable) {
                            Text("Actionable only")
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        .tint(XTheme.accent)
                        .scaleEffect(0.85)
                    }
                }

                if predictions.isEmpty {
                    Text("No GNN predictions available")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 8)
                } else if filtered.isEmpty {
                    Text("No actions needed — all processes are healthy")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.safe)
                        .padding(.vertical, 8)
                } else {
                    // Summary bar
                    if !actionable.isEmpty {
                        actionSummaryBar(actionable: actionable)
                    }

                    // Per-process predictions
                    LazyVStack(spacing: 8) {
                        ForEach(Array(filtered.enumerated()), id: \.element.pid) { _, pred in
                            gnnProcessRow(pred: pred, snapshot: snapshot)
                        }
                    }
                }
            }
        }
    }

    private func actionSummaryBar(actionable: [MemoryGNNManager.MemoryPrediction]) -> some View {
        let counts = Dictionary(grouping: actionable, by: { $0.action })
        return ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 8) {
                ForEach(counts.keys.sorted(), id: \.self) { action in
                    let count = counts[action]?.count ?? 0
                    let color = actionColor(action)
                    HStack(spacing: 4) {
                        Image(systemName: actionIcon(action))
                            .font(.system(size: 10))
                        Text("\(count) \(action.replacingOccurrences(of: "_", with: " "))")
                            .font(.system(size: 10, weight: .medium))
                    }
                    .foregroundStyle(color)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(color.opacity(0.15))
                    .clipShape(Capsule())
                }
            }
        }
    }

    private func gnnProcessRow(pred: MemoryGNNManager.MemoryPrediction, snapshot: OptimizeSnapshot) -> some View {
        // Match by name since graph nodes use labels, not real PIDs
        let proc = snapshot.processes.first { $0.name == pred.processName } ?? snapshot.processes.first { $0.name.lowercased() == pred.processName.lowercased() }
        let color = actionColor(pred.action)
        let icon = actionIcon(pred.action)

        return VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                // Action icon
                Image(systemName: icon)
                    .font(.system(size: 16))
                    .foregroundStyle(color)
                    .frame(width: 28)

                // Process info
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(pred.processName)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                            .lineLimit(1)
                        Text("PID \(pred.pid)")
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(XTheme.textTertiary)
                    }

                    // Action label
                    HStack(spacing: 6) {
                        Text(pred.action.replacingOccurrences(of: "_", with: " ").capitalized)
                            .font(.system(size: 11, weight: .bold))
                            .foregroundStyle(color)
                        Text("(\(Int(pred.actionConfidence * 100))% confidence)")
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                }

                Spacer()

                // Risk + RSS
                VStack(alignment: .trailing, spacing: 2) {
                    if let proc = proc {
                        Text(formatBytes(proc.rssBytes))
                            .font(.system(size: 12, weight: .semibold, design: .monospaced))
                            .foregroundStyle(XTheme.textPrimary)
                    }
                    // Risk bar
                    HStack(spacing: 4) {
                        Text("risk")
                            .font(.system(size: 9))
                            .foregroundStyle(XTheme.textTertiary)
                        riskBar(risk: pred.risk)
                    }
                }
            }

            // Action explanation
            Text(actionExplanation(pred: pred, proc: proc))
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textSecondary)
                .padding(.leading, 38)

            // Action buttons
            if pred.action != "no_action" {
                HStack(spacing: 8) {
                    actionButton(pred: pred)
                    if proc != nil && (pred.action == "terminate" || pred.action == "suspend") {
                        Button {
                            pendingKill = (pid: pred.pid, name: pred.processName, force: pred.action == "terminate")
                        } label: {
                            HStack(spacing: 4) {
                                Image(systemName: pred.action == "terminate" ? "xmark.octagon.fill" : "pause.circle.fill")
                                    .font(.system(size: 10))
                                Text(pred.action == "terminate" ? "Kill" : "Suspend")
                                    .font(.system(size: 10, weight: .semibold))
                            }
                            .foregroundStyle(.white)
                            .padding(.horizontal, 10)
                            .padding(.vertical, 4)
                            .background(pred.action == "terminate" ? XTheme.danger : XTheme.medium)
                            .clipShape(Capsule())
                        }
                        .buttonStyle(.plain)
                        .confirmationDialog(
                            pred.action == "terminate" ? "Kill \(pred.processName) (PID \(pred.pid))?" : "Suspend \(pred.processName) (PID \(pred.pid))?",
                            isPresented: Binding(
                                get: { pendingKill?.pid == pred.pid },
                                set: { if !$0 { pendingKill = nil } }
                            ),
                            titleVisibility: .visible
                        ) {
                            Button(pred.action == "terminate" ? "Kill Process" : "Suspend Process", role: .destructive) {
                                if let pk = pendingKill {
                                    killProcess(pid: pk.pid, name: pk.name, force: pk.force)
                                }
                                pendingKill = nil
                            }
                            Button("Cancel", role: .cancel) { pendingKill = nil }
                        } message: {
                            Text(pred.action == "terminate"
                                ? "This will send SIGKILL to \(pred.processName). The process will be immediately terminated. Unsaved work may be lost."
                                : "This will send SIGTERM to \(pred.processName). The process may shut down gracefully.")
                        }
                    }
                }
                .padding(.leading, 38)
            }
        }
        .padding(.vertical, 6)
        .padding(.horizontal, 10)
        .background(color.opacity(0.05))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(color.opacity(0.15), lineWidth: 1)
        )
    }

    private func actionButton(pred: MemoryGNNManager.MemoryPrediction) -> some View {
        let color = actionColor(pred.action)
        return Button {
            // Copy recommendation to clipboard
            #if os(macOS)
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString("xmac ram-boost --kill-name \"\(pred.processName)\"", forType: .string)
            #endif
        } label: {
            HStack(spacing: 4) {
                Image(systemName: "doc.on.doc")
                    .font(.system(size: 10))
                Text("Copy kill cmd")
                    .font(.system(size: 10, weight: .medium))
            }
            .foregroundStyle(color)
            .padding(.horizontal, 10)
            .padding(.vertical, 4)
            .background(color.opacity(0.1))
            .clipShape(Capsule())
        }
        .buttonStyle(.plain)
    }

    private func riskBar(risk: Double) -> some View {
        let color = risk > 0.7 ? XTheme.danger : risk > 0.4 ? XTheme.medium : XTheme.safe
        return GeometryReader { geo in
            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: 2)
                    .fill(XTheme.bgTertiary)
                    .frame(width: 50, height: 4)
                RoundedRectangle(cornerRadius: 2)
                    .fill(color)
                    .frame(width: 50 * min(max(risk, 0), 1), height: 4)
            }
        }
        .frame(width: 50, height: 4)
    }

    private func actionExplanation(pred: MemoryGNNManager.MemoryPrediction, proc: OptimizeProcess?) -> String {
        switch pred.action {
        case "no_action":
            if let p = proc {
                return "Healthy. RSS \(formatMB(p.rssMB)), \(p.threadCount) threads. No intervention needed."
            }
            return "No intervention needed."
        case "pressure_relief":
            if let p = proc {
                let purgeable = formatMB(Double(p.purgeableBytes) / 1_048_576)
                return "Has \(purgeable) purgeable memory. Reclaiming inactive pages will reduce pressure."
            }
            return "Reclaiming inactive pages from this process will reduce memory pressure."
        case "suggest_purge":
            if let p = proc {
                return "Growing process (\(formatMB(p.rssMB)) RSS). Consider purging its inactive memory or restarting."
            }
            return "Consider purging this process's inactive memory."
        case "deprioritize":
            return "Low priority. Can be deprioritized to give CPU/memory to more important work."
        case "suspend":
            return "Memory-heavy and inactive. Suspending will free RAM for active workloads."
        case "terminate":
            if let p = proc {
                return "High memory usage (\(formatMB(p.rssMB))) with elevated risk. Terminating will reclaim significant RAM."
            }
            return "Terminating this process will reclaim significant memory."
        default:
            return pred.action
        }
    }

    // MARK: - Process List Card

    private func processListCard(snapshot: OptimizeSnapshot) -> some View {
        let sorted = snapshot.processes.sorted { $0.rssBytes > $1.rssBytes }

        return XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Top Memory Consumers", icon: "flame.fill", count: sorted.count)

                if sorted.isEmpty {
                    Text("No process data available")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 8)
                } else {
                    LazyVStack(spacing: 4) {
                        ForEach(Array(sorted.prefix(10).enumerated()), id: \.element.id) { idx, proc in
                            processDetailRow(rank: idx + 1, proc: proc, snapshot: snapshot)
                        }
                    }
                }
            }
        }
    }

    private func processDetailRow(rank: Int, proc: OptimizeProcess, snapshot: OptimizeSnapshot) -> some View {
        let pred = proc.gnnPrediction(from: snapshot.gnnResult)
        let actionColor = pred != nil ? actionColor(pred!.action) : XTheme.textTertiary

        return HStack(spacing: 10) {
            Text("#\(rank)")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(XTheme.textTertiary)
                .frame(width: 24)

            Image(systemName: procIcon(proc.name))
                .font(.system(size: 14))
                .foregroundStyle(actionColor.opacity(0.7))
                .frame(width: 20)

            VStack(alignment: .leading, spacing: 2) {
                Text(proc.name)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                    .lineLimit(1)
                HStack(spacing: 6) {
                    Text("PID \(proc.pid)")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(XTheme.textTertiary)
                    if proc.isSystem {
                        Text("SYS")
                            .font(.system(size: 8, weight: .bold))
                            .foregroundStyle(XTheme.textTertiary)
                            .padding(.horizontal, 4)
                            .padding(.vertical, 1)
                            .background(XTheme.bgTertiary)
                            .clipShape(Capsule())
                    }
                    if let pred = pred, pred.action != "no_action" {
                        Text(pred.action.replacingOccurrences(of: "_", with: " "))
                            .font(.system(size: 8, weight: .bold))
                            .foregroundStyle(actionColor)
                            .padding(.horizontal, 4)
                            .padding(.vertical, 1)
                            .background(actionColor.opacity(0.15))
                            .clipShape(Capsule())
                    }
                }
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text(formatBytes(proc.rssBytes))
                    .font(.system(size: 12, weight: .semibold, design: .monospaced))
                    .foregroundStyle(XTheme.textPrimary)
                Text("\(proc.threadCount) threads")
                    .font(.system(size: 9))
                    .foregroundStyle(XTheme.textTertiary)
            }
        }
        .padding(.vertical, 4)
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundStyle(XTheme.cardBorder)
                .padding(.leading, 34),
            alignment: .bottom
        )
    }

    // MARK: - Boost Controls

    private var boostControlsCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 14) {
                XSectionHeader(title: "Manual Controls", icon: "bolt.fill", count: nil)

                // Purge toggle
                Toggle(isOn: $purgeEnabled) {
                    Label("Purge inactive memory", systemImage: "arrow.triangle.2.circlepath")
                        .font(.system(size: 13))
                }
                .tint(XTheme.accent)

                if purgeEnabled {
                    HStack(spacing: 6) {
                        Image(systemName: "lock.fill")
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.accent)
                        Text("Admin privileges required — macOS will prompt for your password")
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    .padding(.leading, 4)
                }

                Divider().background(XTheme.cardBorder)

                // Kill top N
                VStack(alignment: .leading, spacing: 6) {
                    Label("Kill top memory-hungry processes", systemImage: "xmark.circle")
                        .font(.system(size: 13))
                    Stepper(value: $killTopN, in: 0...10) {
                        Text(killTopN == 0 ? "None (safe)" : "Top \(killTopN) processes")
                            .font(.system(size: 12))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .tint(XTheme.accent)
                }

                // Kill by name
                VStack(alignment: .leading, spacing: 6) {
                    Label("Kill by process name", systemImage: "magnifyingglass")
                        .font(.system(size: 13))
                    TextField("e.g. Slack,Spotify", text: $killByName)
                        .textFieldStyle(.roundedBorder)
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(XTheme.textPrimary)
                }

                // Options
                HStack(spacing: 16) {
                    Toggle(isOn: $protectSystem) {
                        Text("Protect system")
                            .font(.system(size: 11))
                    }
                    .tint(XTheme.accent)

                    Toggle(isOn: $forceKill) {
                        Text("Force kill")
                            .font(.system(size: 11))
                    }
                    .tint(XTheme.danger)
                }

                VStack(alignment: .leading, spacing: 4) {
                    Text("Min RSS: \(minRssMB) MB")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                    Slider(value: Binding(
                        get: { Double(minRssMB) },
                        set: { minRssMB = Int($0) }
                    ), in: 100...4000, step: 100)
                    .tint(XTheme.accent)
                }

                // Boost button
                Button {
                    runBoost()
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "bolt.fill")
                            .font(.system(size: 14))
                        Text(hasRunBoost ? "Boost Again" : "Boost RAM")
                            .font(.system(size: 15, weight: .bold))
                        if purgeEnabled {
                            Image(systemName: "lock.fill")
                                .font(.system(size: 10))
                                .foregroundStyle(.white.opacity(0.7))
                        }
                    }
                    .foregroundStyle(.white)
                    .padding(.horizontal, 24)
                    .padding(.vertical, 10)
                    .background(XTheme.neuralGradient)
                    .clipShape(Capsule())
                    .xGlow(XTheme.accent, radius: 6)
                }
                .buttonStyle(.plain)
                .disabled(isBoosting)
                .frame(maxWidth: .infinity)
            }
        }
    }

    // MARK: - Purge Status Card

    private var purgeStatusCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                if purgeSuccess {
                    HStack(spacing: 8) {
                        Image(systemName: "checkmark.circle.fill")
                            .font(.system(size: 16))
                            .foregroundStyle(XTheme.safe)
                        Text("Memory Purged with Admin Privileges")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(XTheme.safe)
                    }
                    Text(purgeMessage.isEmpty ? "Inactive memory has been purged." : purgeMessage)
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)
                } else {
                    HStack(spacing: 8) {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .font(.system(size: 16))
                            .foregroundStyle(XTheme.medium)
                        Text("Purge Was Not Completed")
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(XTheme.medium)
                    }
                    Text(purgeMessage.isEmpty ? "The purge command was not completed." : purgeMessage)
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)

                    if !purgeMessage.contains("cancelled") {
                        HStack(spacing: 8) {
                            Image(systemName: "terminal")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.accent)
                            Text("sudo purge")
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundStyle(XTheme.textPrimary)
                                .textSelection(.enabled)
                        }
                        .padding(.horizontal, 10)
                        .padding(.vertical, 6)
                        .background(XTheme.bgTertiary)
                        .clipShape(RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }

    // MARK: - Boost Result Card

    private func boostResultCard(before: MemoryStatsSnapshot, after: MemoryStatsSnapshot) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 14) {
                XSectionHeader(title: "Boost Results", icon: "checkmark.seal.fill", count: nil)

                HStack(spacing: 20) {
                    resultBadge(icon: "arrow.down.circle.fill", label: "RAM Freed", value: formatBytes(freedBytes), color: freedBytes > 0 ? XTheme.safe : XTheme.textSecondary)
                    resultBadge(icon: "arrow.down.circle.fill", label: "Swap Freed", value: formatBytes(freedSwapBytes), color: freedSwapBytes > 0 ? XTheme.safe : XTheme.textSecondary)
                    resultBadge(icon: "xmark.circle.fill", label: "Killed", value: "\(processesKilled)", color: processesKilled > 0 ? XTheme.medium : XTheme.textSecondary)
                }

                Divider().background(XTheme.cardBorder)

                HStack(spacing: 16) {
                    usageComparisonBar(label: "Before", used: before.used_bytes, total: before.total_bytes, color: XTheme.medium)
                    usageComparisonBar(label: "After", used: after.used_bytes, total: after.total_bytes, color: XTheme.safe)
                }
            }
        }
    }

    private func resultBadge(icon: String, label: String, value: String, color: Color) -> some View {
        VStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 20))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 16, weight: .bold, design: .rounded))
                .foregroundStyle(color)
            Text(label)
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity)
    }

    private func usageComparisonBar(label: String, used: UInt64, total: UInt64, color: Color) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(label)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textSecondary)
                Spacer()
                Text("\(percentage(used, total))%")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(color)
            }
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 4)
                        .fill(XTheme.bgTertiary)
                        .frame(height: 8)
                    RoundedRectangle(cornerRadius: 4)
                        .fill(color)
                        .frame(width: geo.size.width * ratio(used, total), height: 8)
                }
            }
            .frame(height: 8)
            Text("\(formatBytes(used)) / \(formatBytes(total))")
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(XTheme.textTertiary)
        }
    }

    // MARK: - Helpers

    private func pressureColor(_ level: String) -> Color {
        switch level.lowercased() {
        case let x where x.contains("critical"): return XTheme.danger
        case let x where x.contains("warning") || x.contains("warn"): return XTheme.medium
        default: return XTheme.safe
        }
    }

    private func pressureIcon(_ level: String) -> String {
        switch level.lowercased() {
        case let x where x.contains("critical"): return "exclamationmark.octagon.fill"
        case let x where x.contains("warning") || x.contains("warn"): return "exclamationmark.triangle.fill"
        default: return "checkmark.circle.fill"
        }
    }

    private func actionColor(_ action: String) -> Color {
        switch action {
        case "no_action": return XTheme.safe
        case "pressure_relief": return XTheme.accent
        case "suggest_purge": return XTheme.medium
        case "deprioritize": return XTheme.teal
        case "suspend": return XTheme.medium
        case "terminate": return XTheme.danger
        default: return XTheme.textSecondary
        }
    }

    private func actionIcon(_ action: String) -> String {
        switch action {
        case "no_action": return "checkmark.circle"
        case "pressure_relief": return "arrow.triangle.2.circlepath"
        case "suggest_purge": return "sparkles"
        case "deprioritize": return "arrow.down.right.circle"
        case "suspend": return "pause.circle"
        case "terminate": return "xmark.octagon"
        default: return "questionmark.circle"
        }
    }

    private func statRow(label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textSecondary)
            Spacer()
            Text(value)
                .font(.system(size: 12, design: .monospaced))
                .foregroundStyle(XTheme.textPrimary)
        }
    }

    private func percentage(_ used: UInt64, _ total: UInt64) -> Int {
        guard total > 0 else { return 0 }
        return Int(Double(used) / Double(total) * 100)
    }

    private func ratio(_ used: UInt64, _ total: UInt64) -> Double {
        guard total > 0 else { return 0 }
        return min(Double(used) / Double(total), 1.0)
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let mb = Double(bytes) / 1_048_576
        if mb >= 1024 {
            return String(format: "%.1f GB", mb / 1024)
        }
        if mb >= 1 {
            return String(format: "%.1f MB", mb)
        }
        return String(format: "%.0f KB", Double(bytes) / 1024)
    }

    private func formatMB(_ mb: Double) -> String {
        if mb >= 1024 {
            return String(format: "%.1f GB", mb / 1024)
        }
        return String(format: "%.1f MB", mb)
    }

    private func procIcon(_ name: String) -> String {
        let lower = name.lowercased()
        if lower.contains("chrome") { return "globe" }
        if lower.contains("safari") { return "safari" }
        if lower.contains("firefox") { return "flame" }
        if lower.contains("spotify") { return "music.note" }
        if lower.contains("slack") { return "message" }
        if lower.contains("xcode") { return "hammer" }
        if lower.contains("finder") { return "folder" }
        if lower.contains("terminal") || lower.contains("iterm") { return "terminal" }
        if lower.contains("docker") { return "shippingbox" }
        if lower.contains("node") { return "node" }
        if lower.contains("python") { return "python" }
        if lower.contains("rust") || lower.contains("cargo") { return "rust" }
        if lower.contains("kernel") { return "gearshape.2" }
        if lower.contains("windowserver") { return "rectangle.on.rectangle" }
        if lower.contains("devin") { return "cpu" }
        if lower.contains("chatgpt") { return "bubble.left" }
        if lower.contains("raycast") { return "magnifyingglass.circle" }
        if lower.contains("mail") { return "envelope" }
        if lower.contains("music") { return "music.note" }
        if lower.contains("photos") { return "photo" }
        if lower.contains("code") { return "chevron.left.forwardslash.chevron.right" }
        return "app"
    }

    // MARK: - Actions

    private func killProcess(pid: Int, name: String, force: Bool) {
        Task {
            await runner.killProcess(pid: pid, force: force)
            // Refresh after a short delay
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            await runner.runOptimize(topN: 15)
        }
    }

    private func runBoost() {
        isBoosting = true
        boostError = nil
        hasRunBoost = true
        let killNames = killByName.trimmingCharacters(in: .whitespaces).isEmpty ? nil : killByName
        Task {
            let result = await runner.runRamBoost(
                purge: purgeEnabled,
                killTop: killTopN,
                killName: killNames,
                force: forceKill,
                minRssMB: minRssMB,
                protectSystem: protectSystem
            )
            await MainActor.run {
                isBoosting = false
                switch result {
                case .success(let (before, after, freed, freedSwap, killed, pSuccess, pMessage)):
                    beforeStats = before
                    afterStats = after
                    freedBytes = freed
                    freedSwapBytes = freedSwap
                    processesKilled = killed
                    purgeSuccess = pSuccess
                    purgeMessage = pMessage
                    // Refresh the GNN view
                    Task { await runner.runOptimize(topN: 15) }
                case .failure(let err):
                    boostError = err.localizedDescription
                }
            }
        }
    }
}

// MARK: - ProcessMemoryEntry icon extension

extension ProcessMemoryEntry {
    var icon: String {
        let lower = name.lowercased()
        if lower.contains("chrome") { return "globe" }
        if lower.contains("safari") { return "safari" }
        if lower.contains("firefox") { return "flame" }
        if lower.contains("spotify") { return "music.note" }
        if lower.contains("slack") { return "message" }
        if lower.contains("xcode") { return "hammer" }
        if lower.contains("finder") { return "folder" }
        if lower.contains("terminal") || lower.contains("iterm") { return "terminal" }
        if lower.contains("docker") { return "shippingbox" }
        if lower.contains("node") { return "node" }
        if lower.contains("python") { return "python" }
        if lower.contains("rust") || lower.contains("cargo") { return "rust" }
        if lower.contains("kernel") { return "gearshape.2" }
        if lower.contains("windowserver") { return "rectangle.on.rectangle" }
        if lower.contains("devin") { return "cpu" }
        if lower.contains("chatgpt") { return "bubble.left" }
        if lower.contains("raycast") { return "magnifyingglass.circle" }
        if lower.contains("mail") { return "envelope" }
        if lower.contains("music") { return "music.note" }
        if lower.contains("photos") { return "photo" }
        if lower.contains("code") { return "chevron.left.forwardslash.chevron.right" }
        return "app"
    }
}

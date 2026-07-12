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

// MARK: - RamBoostView

struct RamBoostView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var beforeStats: MemoryStatsSnapshot? = nil
    @State private var afterStats: MemoryStatsSnapshot? = nil
    @State private var freedBytes: UInt64 = 0
    @State private var freedSwapBytes: UInt64 = 0
    @State private var processesKilled: Int = 0
    @State private var isBoosting = false
    @State private var boostError: String? = nil
    @State private var hasRunBoost = false

    // Kill options
    @State private var killTopN: Int = 0
    @State private var killByName: String = ""
    @State private var minRssMB: Int = 500
    @State private var protectSystem: Bool = true
    @State private var forceKill: Bool = false

    var body: some View {
        ScrollView {
            VStack(spacing: 18) {
                if isBoosting {
                    boostProgressView
                } else if let err = boostError {
                    ScanErrorView(message: err)
                } else if let before = beforeStats {
                    // Show purge warning if boost ran but freed nothing
                    if let after = afterStats, freedBytes == 0, hasRunBoost {
                        purgeWarningCard
                    }

                    // After-boost summary if available
                    if let after = afterStats {
                        boostResultCard(before: before, after: after)
                    }

                    // Memory pressure gauge
                    pressureGaugeCard(stats: afterStats ?? before)

                    // Memory breakdown
                    memoryBreakdownCard(stats: afterStats ?? before)

                    // Top consumers
                    topConsumersCard(stats: afterStats ?? before)

                    // Boost controls
                    if afterStats == nil {
                        boostControlsCard
                    } else {
                        // Allow re-running
                        boostControlsCard
                    }
                } else {
                    emptyStateCard
                }
            }
            .padding(24)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if beforeStats == nil && !isBoosting {
                runReportOnly()
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

                Text("Purge inactive memory, kill memory-hungry processes, and see before/after comparison.")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
                    .multilineTextAlignment(.center)

                Button {
                    runReportOnly()
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "chart.bar.fill")
                            .font(.system(size: 12))
                        Text("Show Memory Report")
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

    // MARK: - Purge Warning

    private var purgeWarningCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                HStack(spacing: 8) {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 16))
                        .foregroundStyle(XTheme.medium)
                    Text("Purge Requires Elevated Permissions")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.medium)
                }

                Text("The `purge` command needs sudo to free inactive memory on modern macOS. The memory report ran successfully, but no RAM was freed.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)

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

                Text("Run this in Terminal to purge inactive memory. You can also try killing memory-hungry processes below — that works without sudo.")
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textTertiary)
            }
        }
    }

    // MARK: - Boost Progress

    private var boostProgressView: some View {
        XCard {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.2)
                    .tint(XTheme.accent)

                Text("Optimizing Memory...")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)

                Text("Purging inactive memory and reclaiming RAM")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 30)
        }
    }

    // MARK: - Boost Result Card

    private func boostResultCard(before: MemoryStatsSnapshot, after: MemoryStatsSnapshot) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 14) {
                XSectionHeader(title: "Boost Results", icon: "checkmark.seal.fill", count: nil)

                HStack(spacing: 20) {
                    resultBadge(
                        icon: "arrow.down.circle.fill",
                        label: "RAM Freed",
                        value: formatBytes(freedBytes),
                        color: freedBytes > 0 ? XTheme.safe : XTheme.textSecondary
                    )
                    resultBadge(
                        icon: "arrow.down.circle.fill",
                        label: "Swap Freed",
                        value: formatBytes(freedSwapBytes),
                        color: freedSwapBytes > 0 ? XTheme.safe : XTheme.textSecondary
                    )
                    resultBadge(
                        icon: "xmark.circle.fill",
                        label: "Killed",
                        value: "\(processesKilled)",
                        color: processesKilled > 0 ? XTheme.medium : XTheme.textSecondary
                    )
                }

                Divider().background(XTheme.cardBorder)

                // Before / After comparison bar
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

    // MARK: - Pressure Gauge

    private func pressureGaugeCard(stats: MemoryStatsSnapshot) -> some View {
        let pressure = pressureLevel(stats.memory_pressure)
        return XCard {
            VStack(alignment: .leading, spacing: 14) {
                XSectionHeader(title: "Memory Pressure", icon: "gauge.with.dots.needle.67percent")

                HStack(spacing: 16) {
                    // Circular gauge
                    ZStack {
                        Circle()
                            .stroke(XTheme.bgTertiary, lineWidth: 10)
                            .frame(width: 80, height: 80)
                        Circle()
                            .trim(from: 0, to: ratio(stats.used_bytes, stats.total_bytes))
                            .stroke(pressure.color, style: StrokeStyle(lineWidth: 10, lineCap: .round))
                            .rotationEffect(.degrees(-90))
                            .frame(width: 80, height: 80)
                            .xGlow(pressure.color, radius: 4)
                        VStack(spacing: 2) {
                            Text("\(Int(ratio(stats.used_bytes, stats.total_bytes) * 100))%")
                                .font(.system(size: 18, weight: .bold, design: .rounded))
                                .foregroundStyle(pressure.color)
                            Text("Used")
                                .font(.system(size: 9))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        HStack(spacing: 6) {
                            Circle()
                                .fill(pressure.color)
                                .frame(width: 8, height: 8)
                                .xGlow(pressure.color, radius: 3)
                            Text(pressure.label)
                                .font(.system(size: 14, weight: .semibold))
                                .foregroundStyle(pressure.color)
                        }

                        statRow(label: "Total", value: formatBytes(stats.total_bytes))
                        statRow(label: "Used", value: formatBytes(stats.used_bytes))
                        statRow(label: "Available", value: formatBytes(stats.available_bytes))
                    }
                    Spacer()
                }
            }
        }
    }

    // MARK: - Memory Breakdown

    private func memoryBreakdownCard(stats: MemoryStatsSnapshot) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Memory Breakdown", icon: "chart.bar.fill")

                if stats.wired_bytes > 0 {
                    breakdownBar(label: "App Memory", bytes: stats.app_memory_bytes, total: stats.total_bytes, color: XTheme.accent)
                    breakdownBar(label: "Wired", bytes: stats.wired_bytes, total: stats.total_bytes, color: XTheme.teal)
                    breakdownBar(label: "Compressed", bytes: stats.compressed_bytes, total: stats.total_bytes, color: XTheme.anomaly)
                }

                if stats.swap_total_bytes > 0 {
                    Divider().background(XTheme.cardBorder)
                    HStack {
                        Text("Swap")
                            .font(.system(size: 12, weight: .medium))
                            .foregroundStyle(XTheme.textSecondary)
                        Spacer()
                        Text("\(formatBytes(stats.swap_used_bytes)) / \(formatBytes(stats.swap_total_bytes))")
                            .font(.system(size: 12, design: .monospaced))
                            .foregroundStyle(stats.swap_used_bytes > 0 ? XTheme.medium : XTheme.textTertiary)
                    }
                    if stats.swap_total_bytes > 0 {
                        GeometryReader { geo in
                            ZStack(alignment: .leading) {
                                RoundedRectangle(cornerRadius: 3)
                                    .fill(XTheme.bgTertiary)
                                    .frame(height: 6)
                                RoundedRectangle(cornerRadius: 3)
                                    .fill(XTheme.medium)
                                    .frame(width: geo.size.width * ratio(stats.swap_used_bytes, stats.swap_total_bytes), height: 6)
                            }
                        }
                        .frame(height: 6)
                    }
                }
            }
        }
    }

    private func breakdownBar(label: String, bytes: UInt64, total: UInt64, color: Color) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(XTheme.textSecondary)
                Spacer()
                Text(formatBytes(bytes))
                    .font(.system(size: 12, design: .monospaced))
                    .foregroundStyle(XTheme.textPrimary)
            }
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 3)
                        .fill(XTheme.bgTertiary)
                        .frame(height: 6)
                    RoundedRectangle(cornerRadius: 3)
                        .fill(color)
                        .frame(width: geo.size.width * ratio(bytes, total), height: 6)
                }
            }
            .frame(height: 6)
        }
    }

    // MARK: - Top Consumers

    private func topConsumersCard(stats: MemoryStatsSnapshot) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Top Memory Consumers", icon: "flame.fill", count: stats.top_consumers.count)

                if stats.top_consumers.isEmpty {
                    Text("No process data available")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.vertical, 8)
                } else {
                    LazyVStack(spacing: 6) {
                        ForEach(Array(stats.top_consumers.enumerated()), id: \.element.id) { idx, proc in
                            processRow(rank: idx + 1, proc: proc, total: stats.total_bytes)
                        }
                    }
                }
            }
        }
    }

    private func processRow(rank: Int, proc: ProcessMemoryEntry, total: UInt64) -> some View {
        HStack(spacing: 10) {
            Text("#\(rank)")
                .font(.system(size: 11, weight: .bold, design: .monospaced))
                .foregroundStyle(XTheme.textTertiary)
                .frame(width: 24)

            Image(systemName: proc.icon)
                .font(.system(size: 14))
                .foregroundStyle(XTheme.accent.opacity(0.7))
                .frame(width: 20)

            VStack(alignment: .leading, spacing: 2) {
                Text(proc.name)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                    .lineLimit(1)
                Text("PID \(proc.pid)")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(XTheme.textTertiary)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text(formatBytes(proc.rss_bytes))
                    .font(.system(size: 12, weight: .semibold, design: .monospaced))
                    .foregroundStyle(XTheme.textPrimary)
                Text(String(format: "%.1f%%", proc.percent))
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textSecondary)
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
                XSectionHeader(title: "Boost Controls", icon: "bolt.fill")

                // Purge toggle
                Toggle(isOn: $purgeEnabled) {
                    Label("Purge inactive memory", systemImage: "arrow.triangle.2.circlepath")
                        .font(.system(size: 13))
                }
                .tint(XTheme.accent)

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

    // MARK: - Helpers

    @State private var purgeEnabled: Bool = true

    private func pressureLevel(_ s: String) -> (label: String, color: Color) {
        switch s.lowercased() {
        case let x where x.contains("critical"): return ("Critical", XTheme.danger)
        case let x where x.contains("warning"): return ("Warning", XTheme.medium)
        default: return ("Nominal", XTheme.safe)
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

    // MARK: - Actions

    private func runReportOnly() {
        isBoosting = true
        boostError = nil
        Task {
            let result = await runner.runRamBoost(purge: false, killTop: 0, killName: nil, force: false, minRssMB: 500, protectSystem: true)
            await MainActor.run {
                isBoosting = false
                switch result {
                case .success(let (before, _, _, _, _)):
                    beforeStats = before
                    afterStats = nil
                    hasRunBoost = false
                case .failure(let err):
                    boostError = err.localizedDescription
                }
            }
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
                case .success(let (before, after, freed, freedSwap, killed)):
                    beforeStats = before
                    afterStats = after
                    freedBytes = freed
                    freedSwapBytes = freedSwap
                    processesKilled = killed
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

// MARK: - ScanErrorView is defined in FullScanView.swift

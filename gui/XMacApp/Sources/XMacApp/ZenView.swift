import SwiftUI

struct ZenView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var showingOptimizeConfirmation = false

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                heroSection
                if runner.isZenRunning {
                    zenRunningSection
                } else if let result = runner.zenResult {
                    zenResultSection(result)
                } else {
                    zenIdleSection
                }
            }
            .padding(28)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.zenResult == nil && !runner.isZenRunning {
                // Don't auto-run — let the user choose
            }
        }
    }

    // MARK: - Hero

    private var heroSection: some View {
        VStack(spacing: 16) {
            VStack(spacing: 6) {
                Image(systemName: "circle.hexagongrid.fill")
                    .font(.system(size: 48, weight: .light))
                    .foregroundStyle(XTheme.neuralGradient)
                    .xHeroGlow(XTheme.accent, radius: 16)

                Text("Zen Mode")
                    .font(.system(size: 32, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)

                Text("One-click comprehensive optimization")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
            }

            HStack(spacing: 12) {
                Button {
                    runner.runZen(execute: false)
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "eye")
                            .font(.system(size: 16, weight: .semibold))
                        Text("Preview")
                            .font(.system(size: 15, weight: .bold))
                    }
                    .frame(width: 140, height: 48)
                    .foregroundStyle(XTheme.textPrimary)
                    .background(XTheme.cardBg)
                    .overlay(
                        RoundedRectangle(cornerRadius: 24)
                            .stroke(XTheme.cardBorder, lineWidth: 1)
                    )
                    .clipShape(Capsule())
                }
                .buttonStyle(.plain)
                .disabled(runner.isZenRunning)

                Button {
                    showingOptimizeConfirmation = true
                } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "bolt.fill")
                            .font(.system(size: 16, weight: .semibold))
                        Text("Optimize Now")
                            .font(.system(size: 15, weight: .bold))
                    }
                    .frame(width: 180, height: 48)
                    .foregroundStyle(.white)
                    .background(XTheme.neuralGradient)
                    .clipShape(Capsule())
                }
                .buttonStyle(.plain)
                .xHeroGlow(XTheme.accent, radius: 12)
                .disabled(runner.isZenRunning)
                .confirmationDialog(
                    "Run Zen Mode optimization?",
                    isPresented: $showingOptimizeConfirmation,
                    titleVisibility: .visible
                ) {
                    Button("Optimize Now", role: .destructive) {
                        runner.runZen(execute: true)
                    }
                    Button("Cancel", role: .cancel) {}
                } message: {
                    Text("This will execute maintenance tasks, purge inactive memory, and optimize system performance. Some changes may take a few moments to take effect.")
                }
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 16)
    }

    // MARK: - Running

    private var zenRunningSection: some View {
        XCard {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.2)
                Text("Optimizing your system...")
                    .font(.system(size: 16, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                Text("Running clean scan, memory optimization, and maintenance")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
        }
    }

    // MARK: - Idle

    private var zenIdleSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 16) {
                XSectionHeader(title: "What Zen Mode does", icon: "list.bullet")

                ZenFeatureRow(icon: "trash.circle", title: "Disk Cleanup",
                    description: "Scans for caches, build artifacts, browser data, and other reclaimable space")
                ZenFeatureRow(icon: "bolt.circle", title: "Memory Optimization",
                    description: "Purges inactive memory to free up RAM for active applications")
                ZenFeatureRow(icon: "wrench.and.screwdriver", title: "Safe Maintenance",
                    description: "Flushes DNS, reindexes Spotlight, clears Quick Look cache, and more")
                ZenFeatureRow(icon: "chart.bar", title: "Health Score",
                    description: "Measures system health before and after to show the improvement")

                Spacer().frame(height: 8)

                Text("Preview shows what would be done without making changes. Optimize Now executes the full optimization.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textTertiary)
            }
        }
    }

    // MARK: - Result

    private func zenResultSection(_ result: ZenResult) -> some View {
        VStack(spacing: 16) {
            // Health score card
            XCard {
                VStack(spacing: 16) {
                    HStack(spacing: 40) {
                        VStack(spacing: 4) {
                            Text("Before")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                            Text("\(Int(result.health_before))")
                                .font(.system(size: 36, weight: .bold, design: .rounded))
                                .foregroundStyle(XTheme.textPrimary)
                            Text("/ 100")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textTertiary)
                        }

                        Image(systemName: "arrow.right")
                            .font(.system(size: 20))
                            .foregroundStyle(XTheme.accent)

                        VStack(spacing: 4) {
                            Text("After")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                            Text("\(Int(result.health_after))")
                                .font(.system(size: 36, weight: .bold, design: .rounded))
                                .foregroundStyle(healthColor(result.health_after))
                            Text("/ 100")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textTertiary)
                        }

                        Spacer()

                        VStack(spacing: 4) {
                            let delta = result.health_after - result.health_before
                            Text("Change")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                            Text(delta >= 0 ? "+\(Int(delta))" : "\(Int(delta))")
                                .font(.system(size: 28, weight: .bold, design: .rounded))
                                .foregroundStyle(delta >= 0 ? XTheme.safe : XTheme.danger)
                        }
                    }

                    Text(String(format: "Completed in %.1fs", result.duration_secs))
                        .font(.system(size: 12, design: .monospaced))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }

            // Memory card
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Memory", icon: "bolt.fill")

                    let memFreed = result.memory_before.used_bytes > result.memory_after.used_bytes
                        ? result.memory_before.used_bytes - result.memory_after.used_bytes : 0

                    HStack(spacing: 20) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Usage")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                            Text("\(formatBytes(result.memory_before.used_bytes)) → \(formatBytes(result.memory_after.used_bytes))")
                                .font(.system(size: 14, weight: .medium, design: .monospaced))
                                .foregroundStyle(XTheme.textPrimary)
                        }

                        if memFreed > 0 {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Freed")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                Text(formatBytes(memFreed))
                                    .font(.system(size: 14, weight: .bold, design: .monospaced))
                                    .foregroundStyle(XTheme.safe)
                            }
                        }

                        Spacer()

                        // Utilization gauge
                        ZStack {
                            Circle()
                                .stroke(XTheme.cardBorder, lineWidth: 4)
                                .frame(width: 56, height: 56)
                            Circle()
                                .trim(from: 0, to: result.memory_after.utilization)
                                .stroke(healthColor(1.0 - result.memory_after.utilization), lineWidth: 4)
                                .frame(width: 56, height: 56)
                                .rotationEffect(.degrees(-90))
                            Text("\(Int(result.memory_after.utilization * 100))%")
                                .font(.system(size: 12, weight: .bold, design: .rounded))
                                .foregroundStyle(XTheme.textPrimary)
                        }
                    }
                }
            }

            // Disk card
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Disk", icon: "internaldrive")

                    if result.reclaimed_bytes > 0 {
                        HStack(spacing: 20) {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Reclaimed")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                Text(formatBytes(result.reclaimed_bytes))
                                    .font(.system(size: 18, weight: .bold, design: .rounded))
                                    .foregroundStyle(XTheme.safe)
                            }
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Total reclaimable found")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                Text(formatBytes(result.reclaimable_bytes))
                                    .font(.system(size: 14, weight: .medium, design: .monospaced))
                                    .foregroundStyle(XTheme.textPrimary)
                            }
                            Spacer()
                        }
                    } else {
                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Reclaimable space found")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                Text(formatBytes(result.reclaimable_bytes))
                                    .font(.system(size: 18, weight: .bold, design: .rounded))
                                    .foregroundStyle(XTheme.accent)
                            }
                            Spacer()
                            Text("\(result.findings_count) items")
                                .font(.system(size: 13))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                    }

                    if !result.top_categories.isEmpty {
                        VStack(alignment: .leading, spacing: 6) {
                            Text("Top categories")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                            ForEach(result.top_categories) { entry in
                                HStack {
                                    Text(entry.category)
                                        .font(.system(size: 12))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Spacer()
                                    Text(formatBytes(entry.size))
                                        .font(.system(size: 12, design: .monospaced))
                                        .foregroundStyle(XTheme.textSecondary)
                                }
                            }
                        }
                    }
                }
            }

            // Steps card
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Steps", icon: "checklist")

                    ForEach(result.steps) { step in
                        HStack(spacing: 10) {
                            Image(systemName: stepIcon(step.status))
                                .font(.system(size: 14, weight: .medium))
                                .foregroundStyle(stepColor(step.status))

                            VStack(alignment: .leading, spacing: 2) {
                                Text(step.name)
                                    .font(.system(size: 13, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text(step.detail)
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                            }

                            Spacer()
                        }
                    }
                }
            }
        }
    }

    // MARK: - Helpers

    private func healthColor(_ score: Double) -> Color {
        if score >= 85 { return XTheme.safe }
        if score >= 70 { return XTheme.accent }
        if score >= 50 { return XTheme.medium }
        if score >= 30 { return XTheme.high }
        return XTheme.critical
    }

    private func stepIcon(_ status: String) -> String {
        switch status {
        case "done": return "checkmark.circle.fill"
        case "preview": return "eye"
        case "scanned": return "eye"
        case "skipped": return "circle.dashed"
        default: return "circle"
        }
    }

    private func stepColor(_ status: String) -> Color {
        switch status {
        case "done": return XTheme.safe
        case "preview", "scanned": return XTheme.accent
        case "skipped": return XTheme.textTertiary
        default: return XTheme.textTertiary
        }
    }
}

private struct ZenFeatureRow: View {
    let icon: String
    let title: String
    let description: String

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 18))
                .foregroundStyle(XTheme.accent)
                .frame(width: 24)
                .xGlow(XTheme.accent, radius: 3)
            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(description)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }
}

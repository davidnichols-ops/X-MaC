import SwiftUI

struct DashboardView: View {
    @EnvironmentObject var runner: XMacRunner
    @EnvironmentObject var settings: AppSettings

    private var reclaimable: Int {
        runner.findings.compactMap { $0.size_bytes }.reduce(0, +)
    }

    private var safeNeuralCount: Int {
        runner.gnnScores.filter { $0.safety_score >= 0.7 }.count
    }

    private var hasScan: Bool {
        !runner.findings.isEmpty || runner.scanDuration > 0
    }

    private var recommendation: Recommendation {
        if !hasScan {
            return Recommendation(
                text: "Run a Quick Clean scan — takes under 30 seconds.",
                action: { runner.startCleanScan() },
                label: "Start"
            )
        }
        if safeNeuralCount > 0 {
            return Recommendation(
                text: "Neural scan found \(safeNeuralCount) safe-to-remove items.",
                action: { runner.startNeuralScan() },
                label: "Review"
            )
        }
        if !runner.findings.isEmpty {
            return Recommendation(
                text: "You have \(formatBytes(reclaimable)) reclaimable. Tap Clean to review items.",
                action: { runner.startCleanScan() },
                label: "Go to Clean"
            )
        }
        return Recommendation(
            text: "System looks healthy. Run a new scan to refresh.",
            action: { runner.startCleanScan() },
            label: "Scan"
        )
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 18) {
                heroSection
                metricCards
                recommendationCard
                protectionCard
            }
            .padding(28)
        }
        .background(XTheme.voidGradient)
    }

    // MARK: - Hero

    private var heroSection: some View {
        VStack(spacing: 16) {
            VStack(spacing: 6) {
                if hasScan {
                    Text(formatBytes(reclaimable))
                        .font(.system(size: 48, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.metallicGradient)
                        .xGlow(XTheme.accent, radius: 6)
                    Text("found across \(runner.findings.count) items")
                        .font(.system(size: 13))
                        .foregroundStyle(XTheme.textSecondary)
                } else {
                    Text("Run a scan to see your storage health")
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(XTheme.textSecondary)
                }
            }
            .frame(minHeight: 80)

            Button {
                runner.startCleanScan()
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "sparkles.rectangle.stack")
                        .font(.system(size: 18, weight: .semibold))
                    Text("Quick Clean")
                        .font(.system(size: 16, weight: .bold))
                }
                .frame(width: 280, height: 52)
                .foregroundStyle(.white)
                .background(XTheme.neuralGradient)
                .clipShape(Capsule())
            }
            .buttonStyle(.plain)
            .xHeroGlow(XTheme.accent, radius: 14)

            HStack(spacing: 12) {
                SecondaryHeroButton(
                    title: "Neural Scan",
                    icon: "brain",
                    color: XTheme.anomaly
                ) {
                    runner.startNeuralScan()
                }

                SecondaryHeroButton(
                    title: "Full Scan",
                    icon: "list.bullet",
                    color: XTheme.accent
                ) {
                    runner.startFullScan()
                }
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 16)
    }

    // MARK: - Status bar

    private var metricCards: some View {
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {
            DashboardMetric(title: "Reclaimable", value: formatBytes(reclaimable), icon: "arrow.down.circle", color: XTheme.safe)
            DashboardMetric(title: "Findings", value: "\(runner.findings.count)", icon: "list.bullet.rectangle", color: XTheme.accent)
            DashboardMetric(title: "Last scan", value: runner.scanDuration > 0 ? String(format: "%.1fs", runner.scanDuration) : "Not run", icon: "clock", color: XTheme.textSecondary)
            DashboardMetric(title: "Neural safe", value: "\(safeNeuralCount)", icon: "brain", color: XTheme.anomaly)
        }
    }

    // MARK: - Recommended next step

    private var recommendationCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 8) {
                    Image(systemName: "lightbulb")
                        .foregroundStyle(XTheme.warning)
                    Text("Recommended next step")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                    Spacer()
                }

                HStack(alignment: .center, spacing: 12) {
                    Text(recommendation.text)
                        .font(.system(size: 13))
                        .foregroundStyle(XTheme.textSecondary)
                        .lineLimit(2)

                    Spacer()

                    Button {
                        recommendation.action()
                    } label: {
                        HStack(spacing: 4) {
                            Text(recommendation.label)
                                .font(.system(size: 12, weight: .semibold))
                            Image(systemName: "arrow.right")
                                .font(.system(size: 10))
                        }
                        .foregroundStyle(XTheme.accent)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(XTheme.accent.opacity(0.12))
                        .clipShape(Capsule())
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    // MARK: - Protection

    private var protectionCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "How X-MaC protects you", icon: "shield.checkered")
                ProtectionRow(icon: "trash", title: "Trash first, permanent delete never by default", description: "Cleanup uses the macOS Trash workflow and blocks system paths.")
                ProtectionRow(icon: "eye", title: "Every action is explainable", description: "Each finding shows its category, size, path, and reason before anything changes.")
                ProtectionRow(icon: "brain", title: "GNN is advisory, not authoritative", description: "Neural scores prioritize review; they never bypass safety policy or user confirmation.")
            }
        }
    }
}

// MARK: - Recommendation model

private struct Recommendation {
    let text: String
    let action: () -> Void
    let label: String
}

// MARK: - Hero secondary button

private struct SecondaryHeroButton: View {
    let title: String
    let icon: String
    let color: Color
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundStyle(color)
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Spacer()
                Image(systemName: "chevron.right")
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textTertiary)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .frame(width: 180, alignment: .leading)
            .background(XTheme.cardBg)
            .overlay(
                RoundedRectangle(cornerRadius: 10)
                    .stroke(XTheme.cardBorder, lineWidth: 1)
            )
            .clipShape(RoundedRectangle(cornerRadius: 10))
        }
        .buttonStyle(.bordered)
    }
}

private struct DashboardMetric: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                Image(systemName: icon)
                    .foregroundStyle(color)
                    .xGlow(color, radius: 3)
                Text(value)
                    .font(.system(size: 22, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.textPrimary)
                Text(title)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

private struct ProtectionRow: View {
    let icon: String
    let title: String
    let description: String

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: icon).foregroundStyle(XTheme.accent).frame(width: 18)
            VStack(alignment: .leading, spacing: 3) {
                Text(title).font(.system(size: 12, weight: .semibold)).foregroundStyle(XTheme.textPrimary)
                Text(description).font(.system(size: 11)).foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }
}

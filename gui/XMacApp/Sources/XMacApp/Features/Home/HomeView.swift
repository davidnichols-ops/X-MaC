import SwiftUI

// MARK: - HomeView
//
// The Home screen is a decision surface, not a dashboard.
// It answers, in order:
//   1. Is my Mac okay?
//   2. What needs my attention?
//   3. What changed?
//   4. What should I do?
//   5. What happened after I acted?

struct HomeView: View {
    @EnvironmentObject var router: AppRouter
    @EnvironmentObject var runner: XMacRunner
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @State private var loadTask: Task<Void, Never>?
    @State private var hasLoadedTwin = false

    var body: some View {
        ScrollView {
            VStack(spacing: XTheme.spacing.sectionSpacing) {
                // 1. Is my Mac okay?
                statusSummary

                // 2. What needs my attention?
                if !topAlerts.isEmpty {
                    attentionSection
                }

                // 4. What should I do?
                recommendedActionsSection

                // 3. What changed?
                recentChangesSection

                // 5. What happened after I acted?
                if !runner.activityLog.isEmpty {
                    recentActivitySection
                }
            }
            .padding(XTheme.spacing.xl)
            .frame(maxWidth: 760)
            .frame(maxWidth: .infinity)
        }
        .background(XTheme.gradient.voidBackground)
        .task {
            await loadData()
        }
        .onDisappear {
            loadTask?.cancel()
        }
    }

    // MARK: - Data Loading

    private func loadData() async {
        guard !hasLoadedTwin else { return }
        hasLoadedTwin = true
        loadTask = Task {
            await runner.collectTwin()
            await runner.queryWhatChanged(since: "24h")
        }
    }

    // MARK: - 1. Status Summary

    private var healthScore: Double {
        runner.digitalTwin?.health_score ?? 0
    }

    private var trustScore: Double {
        runner.digitalTwin?.trust_score ?? 0
    }

    private var statusText: String {
        if runner.twinLoading { return "Checking your Mac…" }
        if runner.twinError != nil { return "Unable to assess system health" }
        if healthScore >= 85 { return "Your Mac is in good condition" }
        if healthScore >= 70 { return "Your Mac is mostly healthy" }
        if healthScore >= 50 { return "Your Mac needs attention" }
        if healthScore > 0 { return "Your Mac needs immediate attention" }
        return "Run a scan to assess your Mac"
    }

    private var statusColor: Color {
        if runner.twinError != nil { return XTheme.color.statusDanger }
        if healthScore >= 85 { return XTheme.color.statusSafe }
        if healthScore >= 70 { return XTheme.color.accent }
        if healthScore >= 50 { return XTheme.color.statusWarning }
        if healthScore > 0 { return XTheme.color.statusDanger }
        return XTheme.color.textTertiary
    }

    private var reclaimableBytes: UInt64 {
        let total = runner.findings.compactMap { $0.size_bytes }.reduce(0, +)
        return UInt64(total)
    }

    private var findingsCount: Int {
        runner.findings.count
    }

    private var statusSummary: some View {
        VStack(spacing: XTheme.spacing.md) {
            // Health ring
            ZStack {
                Circle()
                    .stroke(XTheme.color.cardBorder, lineWidth: 6)
                    .frame(width: 80, height: 80)

                Circle()
                    .trim(from: 0, to: runner.twinLoading ? 0 : CGFloat(healthScore / 100))
                    .stroke(statusColor, style: StrokeStyle(lineWidth: 6, lineCap: .round))
                    .rotationEffect(.degrees(-90))
                    .frame(width: 80, height: 80)
                    .if(!reduceMotion) { $0.animation(.easeInOut(duration: 0.6), value: healthScore) }

                VStack(spacing: 0) {
                    Text(runner.twinLoading ? "…" : "\(Int(healthScore))")
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.color.textPrimary)
                    Text("health")
                        .font(XTheme.typography.label)
                        .foregroundStyle(XTheme.color.textTertiary)
                }
            }
            .accessibilityElement(children: .combine)
            .accessibilityLabel("System health score: \(Int(healthScore)) out of 100")

            Text(statusText)
                .font(XTheme.typography.heroSubtitle)
                .foregroundStyle(XTheme.color.textPrimary)
                .multilineTextAlignment(.center)

            // Sub-scores
            if runner.digitalTwin != nil {
                HStack(spacing: XTheme.spacing.xxl) {
                    subScore(label: "Trust", value: trustScore)
                    if reclaimableBytes > 0 {
                        subScoreLabel(label: "Reclaimable", value: formatBytes(reclaimableBytes))
                    }
                    if findingsCount > 0 {
                        subScoreLabel(label: "Findings", value: "\(findingsCount)")
                    }
                }
                .padding(.top, XTheme.spacing.xs)
            }

            // Primary actions
            HStack(spacing: XTheme.spacing.md) {
                if findingsCount == 0 && !runner.isScanning {
                    XButton("Run Quick Scan", icon: "bolt.fill") {
                        router.navigate(to: .improveQuickScan)
                        runner.startCleanScan()
                    }
                } else if findingsCount > 0 {
                    XButton("Review \(findingsCount) Findings", icon: "trash.circle") {
                        router.navigate(to: .improveClean)
                    }
                }

                XSecondaryButton("What Changed?", icon: "arrow.triangle.2.circlepath") {
                    router.navigate(to: .historyWhatChanged)
                }
            }
            .padding(.top, XTheme.spacing.sm)

            // Last updated
            if !runner.twinLoading {
                Text("Updated \(relativeTime(runner.digitalTwin?.timestamp_ms))")
                    .font(XTheme.typography.micro)
                    .foregroundStyle(XTheme.color.textTertiary)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, XTheme.spacing.xl)
    }

    // MARK: - 2. Attention Section

    private var topAlerts: [Finding] {
        runner.findings
            .filter { $0.severity.lowercased() == "critical" || $0.severity.lowercased() == "high" }
            .sorted { severityRank($0.severity) > severityRank($1.severity) }
            .prefix(3)
            .map { $0 }
    }

    private var attentionSection: some View {
        VStack(alignment: .leading, spacing: XTheme.spacing.md) {
            XSectionHeader(title: "Needs Attention", icon: "exclamationmark.triangle.fill", count: topAlerts.count)

            VStack(spacing: XTheme.spacing.sm) {
                ForEach(topAlerts) { finding in
                    AlertCard(finding: finding) {
                        router.navigate(to: .improveClean)
                    }
                }
            }
        }
    }

    // MARK: - 3. Recommended Actions

    private var recommendedActionsSection: some View {
        VStack(alignment: .leading, spacing: XTheme.spacing.md) {
            XSectionHeader(title: "Recommended Actions", icon: "lightbulb.fill")

            if runner.isScanning {
                HStack(spacing: XTheme.spacing.sm) {
                    ProgressView()
                        .controlSize(.small)
                        .tint(XTheme.color.accent)
                    Text("Scanning — \(runner.scanPhase)…")
                        .font(XTheme.typography.body)
                        .foregroundStyle(XTheme.color.textSecondary)
                }
                .padding(XTheme.spacing.md)
            } else if findingsCount == 0 && runner.digitalTwin == nil {
                Text("Run a scan to get personalized recommendations.")
                    .font(XTheme.typography.body)
                    .foregroundStyle(XTheme.color.textSecondary)
            } else if findingsCount == 0 {
                Text("No recommendations at this time. Your Mac looks healthy.")
                    .font(XTheme.typography.body)
                    .foregroundStyle(XTheme.color.textSecondary)
            } else {
                VStack(spacing: XTheme.spacing.sm) {
                    if reclaimableBytes > 0 {
                        RecommendationCard(
                            problem: "\(formatBytes(reclaimableBytes)) can be reclaimed",
                            evidence: "\(findingsCount) items found by scan engines",
                            benefit: "Free up disk space",
                            risk: "Low — items move to Trash",
                            reversible: true,
                            actionLabel: "Review Items",
                            icon: "trash.circle",
                            color: XTheme.color.statusSafe
                        ) {
                            router.navigate(to: .improveClean)
                        }
                    }

                    RecommendationCard(
                        problem: "Check what changed recently",
                        evidence: "System events are tracked by observers",
                        benefit: "Catch unexpected changes early",
                        risk: "None — read-only",
                        reversible: true,
                        actionLabel: "View Changes",
                        icon: "arrow.triangle.2.circlepath",
                        color: XTheme.color.accent
                    ) {
                        router.navigate(to: .historyWhatChanged)
                    }

                    RecommendationCard(
                        problem: "Run neural scan for AI-powered analysis",
                        evidence: "GNN model predicts file safety scores",
                        benefit: "Find items rule-based scans might miss",
                        risk: "None — analysis only",
                        reversible: true,
                        actionLabel: "Start Neural Scan",
                        icon: "brain",
                        color: XTheme.color.statusAnomaly
                    ) {
                        router.navigate(to: .improveNeuralScan)
                        runner.startNeuralScan()
                    }
                }
            }
        }
    }

    // MARK: - 4. Recent Changes

    private var recentChangesSection: some View {
        VStack(alignment: .leading, spacing: XTheme.spacing.md) {
            XSectionHeader(title: "Recent Changes", icon: "clock.fill")

            if runner.whatChangedLoading {
                HStack(spacing: XTheme.spacing.sm) {
                    ProgressView()
                        .controlSize(.small)
                        .tint(XTheme.color.accent)
                    Text("Loading changes…")
                        .font(XTheme.typography.body)
                        .foregroundStyle(XTheme.color.textSecondary)
                }
                .padding(XTheme.spacing.md)
            } else if let report = runner.whatChanged {
                if report.total_events == 0 {
                    Text("No changes detected in the last 24 hours.")
                        .font(XTheme.typography.body)
                        .foregroundStyle(XTheme.color.textTertiary)
                        .padding(XTheme.spacing.md)
                } else {
                    VStack(spacing: XTheme.spacing.sm) {
                        if !report.new_applications.isEmpty {
                            ChangeRow(
                                icon: "app.badge",
                                color: XTheme.color.statusSafe,
                                text: "\(report.new_applications.count) new app\(report.new_applications.count > 1 ? "s" : "") detected"
                            )
                        }
                        if !report.removed_applications.isEmpty {
                            ChangeRow(
                                icon: "app.badge.checkered",
                                color: XTheme.color.textTertiary,
                                text: "\(report.removed_applications.count) app\(report.removed_applications.count > 1 ? "s" : "") removed"
                            )
                        }
                        let growth = report.storage_growth
                        if growth.net_change != 0 {
                            ChangeRow(
                                icon: growth.net_change > 0 ? "arrow.up.circle" : "arrow.down.circle",
                                color: growth.net_change > 0 ? XTheme.color.statusWarning : XTheme.color.statusSafe,
                                text: "Storage \(growth.net_change > 0 ? "increased" : "decreased") by \(formatBytes(UInt64(abs(growth.net_change))))"
                            )
                        }
                        if !report.security_alerts.isEmpty {
                            ChangeRow(
                                icon: "shield.lefthalf.filled",
                                color: XTheme.color.statusDanger,
                                text: "\(report.security_alerts.count) security alert\(report.security_alerts.count > 1 ? "s" : "")"
                            )
                        }
                    }
                }
            } else {
                Text("Change tracking requires observers to be running.")
                    .font(XTheme.typography.body)
                    .foregroundStyle(XTheme.color.textTertiary)
                    .padding(XTheme.spacing.md)
            }
        }
    }

    // MARK: - 5. Recent Activity

    private var recentActivitySection: some View {
        VStack(alignment: .leading, spacing: XTheme.spacing.md) {
            XSectionHeader(title: "Recent Activity", icon: "clock.arrow.circlepath", count: runner.activityLog.count)

            VStack(spacing: XTheme.spacing.xs) {
                ForEach(runner.activityLog.prefix(8)) { entry in
                    HStack(spacing: XTheme.spacing.sm) {
                        Image(systemName: activityIcon(entry.status))
                            .font(.system(size: 10))
                            .foregroundStyle(activityColor(entry.status))

                        Text(entry.operation)
                            .font(XTheme.typography.body)
                            .foregroundStyle(XTheme.color.textPrimary)

                        Spacer()

                        Text(relativeTime(entry.timestamp))
                            .font(XTheme.typography.monoSmall)
                            .foregroundStyle(XTheme.color.textTertiary)
                    }
                    .padding(.vertical, XTheme.spacing.xs)
                    .accessibilityElement(children: .combine)
                    .accessibilityLabel("\(entry.operation): \(entry.status.rawValue), \(relativeTime(entry.timestamp))")
                }
            }
        }
    }

    // MARK: - Helpers

    private func subScore(label: String, value: Double) -> some View {
        VStack(spacing: 2) {
            Text("\(Int(value))")
                .font(.system(size: 16, weight: .semibold, design: .rounded))
                .foregroundStyle(XTheme.color.textPrimary)
            Text(label)
                .font(XTheme.typography.label)
                .foregroundStyle(XTheme.color.textTertiary)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(label): \(Int(value))")
    }

    private func subScoreLabel(label: String, value: String) -> some View {
        VStack(spacing: 2) {
            Text(value)
                .font(.system(size: 16, weight: .semibold, design: .rounded))
                .foregroundStyle(XTheme.color.textPrimary)
            Text(label)
                .font(XTheme.typography.label)
                .foregroundStyle(XTheme.color.textTertiary)
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(label): \(value)")
    }

    private func severityRank(_ severity: String) -> Int {
        switch severity.lowercased() {
        case "critical": return 5
        case "high": return 4
        case "medium": return 3
        case "low": return 2
        case "info": return 1
        default: return 0
        }
    }

    private func activityIcon(_ status: ActivityStatus) -> String {
        switch status {
        case .started: return "circle.dotted"
        case .success: return "checkmark.circle.fill"
        case .warning: return "exclamationmark.triangle.fill"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private func activityColor(_ status: ActivityStatus) -> Color {
        switch status {
        case .started: return XTheme.color.accent
        case .success: return XTheme.color.statusSafe
        case .warning: return XTheme.color.statusWarning
        case .failed: return XTheme.color.statusDanger
        }
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.allowedUnits = [.useKB, .useMB, .useGB, .useTB]
        formatter.countStyle = .file
        return formatter.string(fromByteCount: Int64(bytes))
    }

    private func relativeTime(_ timestampMs: UInt64?) -> String {
        guard let ms = timestampMs, ms > 0 else { return "never" }
        let date = Date(timeIntervalSince1970: TimeInterval(ms) / 1000)
        return date.formatted(.relative(presentation: .named))
    }

    private func relativeTime(_ date: Date) -> String {
        date.formatted(.relative(presentation: .named))
    }
}

// MARK: - Alert Card

private struct AlertCard: View {
    let finding: Finding
    let action: () -> Void

    @State private var showWhy = false

    var body: some View {
        XCard(glow: false) {
            VStack(alignment: .leading, spacing: XTheme.spacing.sm) {
                HStack(spacing: XTheme.spacing.sm) {
                    Image(systemName: severityIcon)
                        .font(.system(size: 16, weight: .medium))
                        .foregroundStyle(severityColor)

                    VStack(alignment: .leading, spacing: 2) {
                        Text(finding.title)
                            .font(XTheme.typography.bodySemibold)
                            .foregroundStyle(XTheme.color.textPrimary)
                        Text(finding.description)
                            .font(XTheme.typography.caption)
                            .foregroundStyle(XTheme.color.textSecondary)
                            .lineLimit(2)
                    }

                    Spacer()

                    XBadge(finding.severity.capitalized, color: severityColor)
                }

                if showWhy {
                    VStack(alignment: .leading, spacing: XTheme.spacing.xs) {
                        if let hint = finding.remediation_hint {
                            Text("Suggestion: \(hint)")
                                .font(XTheme.typography.caption)
                                .foregroundStyle(XTheme.color.textSecondary)
                        }
                        if let rule = finding.safety_rule {
                            Text("Safety rule: \(rule)")
                                .font(XTheme.typography.caption)
                                .foregroundStyle(XTheme.color.textSecondary)
                        }
                        if let explanation = finding.safety_explanation {
                            Text("Classification: \(explanation)")
                                .font(XTheme.typography.caption)
                                .foregroundStyle(XTheme.color.textSecondary)
                        }
                    }
                    .padding(.top, XTheme.spacing.xs)
                    .transition(.opacity)
                }

                HStack(spacing: XTheme.spacing.md) {
                    Button("Review") { action() }
                        .font(XTheme.typography.captionMedium)
                        .foregroundStyle(XTheme.color.accent)
                        .buttonStyle(.plain)
                        .accessibilityLabel("Review \(finding.title)")

                    Button(showWhy ? "Hide details" : "Why am I seeing this?") {
                        withAnimation(.easeInOut(duration: 0.2)) {
                            showWhy.toggle()
                        }
                    }
                    .font(XTheme.typography.caption)
                    .foregroundStyle(XTheme.color.textTertiary)
                    .buttonStyle(.plain)
                    .accessibilityLabel(showWhy ? "Hide details" : "Show explanation for \(finding.title)")
                }
            }
        }
        .accessibilityElement(children: .contain)
    }

    private var severityIcon: String {
        switch finding.severity.lowercased() {
        case "critical": return "exclamationmark.octagon.fill"
        case "high": return "exclamationmark.triangle.fill"
        case "medium": return "exclamationmark.circle"
        case "low": return "info.circle"
        default: return "info.circle"
        }
    }

    private var severityColor: Color {
        XTheme.color.severity(finding.severity)
    }
}

// MARK: - Recommendation Card

private struct RecommendationCard: View {
    let problem: String
    let evidence: String
    let benefit: String
    let risk: String
    let reversible: Bool
    let actionLabel: String
    let icon: String
    let color: Color
    let action: () -> Void

    var body: some View {
        XCard(glow: false) {
            VStack(alignment: .leading, spacing: XTheme.spacing.sm) {
                HStack(spacing: XTheme.spacing.sm) {
                    Image(systemName: icon)
                        .font(.system(size: 18, weight: .medium))
                        .foregroundStyle(color)

                    Text(problem)
                        .font(XTheme.typography.bodySemibold)
                        .foregroundStyle(XTheme.color.textPrimary)

                    Spacer()
                }

                VStack(alignment: .leading, spacing: XTheme.spacing.xxs) {
                    Label(evidence, systemImage: "magnifyingglass")
                        .font(XTheme.typography.caption)
                        .foregroundStyle(XTheme.color.textSecondary)
                    Label(benefit, systemImage: "checkmark.circle")
                        .font(XTheme.typography.caption)
                        .foregroundStyle(XTheme.color.textSecondary)
                    Label(risk, systemImage: "shield")
                        .font(XTheme.typography.caption)
                        .foregroundStyle(XTheme.color.textSecondary)
                    Label(reversible ? "Reversible — moves to Trash" : "Not reversible",
                          systemImage: reversible ? "arrow.uturn.backward" : "exclamationmark")
                        .font(XTheme.typography.caption)
                        .foregroundStyle(reversible ? XTheme.color.statusSafe : XTheme.color.statusDanger)
                }

                XSecondaryButton(actionLabel, action: action)
            }
        }
        .accessibilityElement(children: .contain)
    }
}

// MARK: - Change Row

private struct ChangeRow: View {
    let icon: String
    let color: Color
    let text: String

    var body: some View {
        HStack(spacing: XTheme.spacing.sm) {
            Image(systemName: icon)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(color)
                .frame(width: 20)
            Text(text)
                .font(XTheme.typography.body)
                .foregroundStyle(XTheme.color.textPrimary)
            Spacer()
        }
        .padding(.vertical, XTheme.spacing.xs)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(text)
    }
}

import SwiftUI

// MARK: - Overview View (Mission Control)
// The "mission control" page: health/perf/storage/battery/security scores,
// active alerts, AI recommendations, timeline of recent events.

struct OverviewView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Score cards
                scoreCards

                // Active alerts
                if !findingsForAlerts.isEmpty {
                    activeAlertsSection
                }

                // AI recommendations
                aiRecommendationsSection

                // Recent timeline
                recentTimelineSection
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.digitalTwin == nil { runner.collectTwin() }
        }
    }

    // MARK: - Score Cards

    private var scoreCards: some View {
        LazyVGrid(columns: [
            GridItem(.flexible(), spacing: 12),
            GridItem(.flexible(), spacing: 12),
            GridItem(.flexible(), spacing: 12),
        ], spacing: 12) {
            ScoreCard(title: "System Health", score: runner.digitalTwin?.health_score ?? 0, icon: "heart.fill", color: XTheme.safe)
            ScoreCard(title: "Trust Score", score: runner.digitalTwin?.trust_score ?? 0, icon: "shield.fill", color: XTheme.accent)
            ScoreCard(title: "Memory", score: (1.0 - (runner.digitalTwin?.memory.utilization ?? 0)) * 100, icon: "memorychip.fill", color: XTheme.medium)
            ScoreCard(title: "Storage", score: storageScore, icon: "internaldrive.fill", color: XTheme.accent)
            ScoreCard(title: "Battery", score: batteryScore, icon: "battery.100.fill", color: XTheme.safe)
            ScoreCard(title: "Security", score: securityScore, icon: "lock.shield.fill", color: XTheme.danger)
        }
    }

    private var storageScore: Double {
        guard let twin = runner.digitalTwin else { return 0 }
        if let forecast = twin.filesystem.exhaustion_forecast_days, forecast < 30 {
            return 30
        }
        return 85
    }

    private var batteryScore: Double {
        guard let bat = runner.digitalTwin?.energy.battery else { return 100 }
        return bat.health_pct ?? Double(bat.charge_pct)
    }

    private var securityScore: Double {
        guard let twin = runner.digitalTwin else { return 100 }
        let suspicious = twin.applications.suspicious_apps.count
        let anomalies = twin.processes.anomalies.filter { $0.severity == "high" || $0.severity == "critical" }.count
        return max(0, 100 - Double(suspicious * 10 + anomalies * 5))
    }

    // MARK: - Active Alerts

    private var findingsForAlerts: [Finding] {
        runner.findings.filter { $0.severity == "critical" || $0.severity == "high" }
    }

    private var activeAlertsSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Active Alerts", icon: "exclamationmark.triangle.fill", count: findingsForAlerts.count)
                ForEach(Array(findingsForAlerts.prefix(5))) { finding in
                    HStack(spacing: 10) {
                        Image(systemName: finding.severityIcon)
                            .foregroundStyle(finding.severityColor)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(finding.title)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(finding.description)
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                                .lineLimit(2)
                        }
                        Spacer()
                    }
                }
            }
        }
    }

    // MARK: - AI Recommendations

    private var aiRecommendationsSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "AI Recommendations", icon: "brain.head.profile.fill")
                if let recs = runner.twinRecommendations {
                    RecommendationRow(icon: "trash.fill", title: "Cleanup Impact", description: "\(recs.cleanup_impact.summary) (\(formatBytes(recs.cleanup_impact.space_freed_bytes)) reclaimable)", color: XTheme.accent)
                    ForEach(Array(recs.workflow_changes.prefix(3))) { change in
                        RecommendationRow(icon: "arrow.triangle.swap", title: change.change, description: change.estimated_benefit, color: XTheme.medium)
                    }
                    ForEach(Array(recs.preventive_actions.prefix(3))) { action in
                        RecommendationRow(icon: "shield.fill", title: action.action, description: action.estimated_impact, color: XTheme.safe)
                    }
                } else {
                    Text("Run a scan to get AI recommendations")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
        }
    }

    // MARK: - Recent Timeline

    private var recentTimelineSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Recent Events", icon: "clock.fill")
                if let twin = runner.digitalTwin {
                    timelineEvents(twin)
                } else {
                    Text("No events yet")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
        }
    }

    private func timelineEvents(_ twin: DigitalTwin) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            // Process anomalies
            ForEach(Array(twin.processes.anomalies.prefix(3))) { anomaly in
                TimelineRow(
                    time: "now",
                    icon: "exclamationmark.triangle.fill",
                    color: XTheme.danger,
                    title: anomaly.name,
                    description: anomaly.description
                )
            }
            // Memory leaks
            ForEach(Array(twin.memory.leak_candidates.prefix(2))) { leak in
                TimelineRow(
                    time: "now",
                    icon: "drop.fill",
                    color: XTheme.medium,
                    title: leak.name,
                    description: "Memory leak: growing \(formatBytes(UInt64(leak.growth_rate_bytes_per_min))))/min"
                )
            }
            // Storage warnings
            if let forecast = twin.filesystem.exhaustion_forecast_days, forecast < 30 {
                TimelineRow(
                    time: "now",
                    icon: "externaldrive.fill",
                    color: XTheme.danger,
                    title: "Storage Warning",
                    description: "Disk will be full in ~\(forecast) days"
                )
            }
        }
    }
}

// MARK: - Score Card

struct ScoreCard: View {
    let title: String
    let score: Double
    let icon: String
    let color: Color

    var body: some View {
        XCard {
            VStack(spacing: 10) {
                Image(systemName: icon)
                    .font(.system(size: 22))
                    .foregroundStyle(color)
                    .xGlow(color, radius: 4)

                ZStack {
                    Circle()
                        .stroke(XTheme.color.textTertiary.opacity(0.2), lineWidth: 4)
                    Circle()
                        .trim(from: 0, to: score / 100)
                        .stroke(color, style: StrokeStyle(lineWidth: 4, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                    Text("\(Int(score))")
                        .font(.system(size: 18, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.textPrimary)
                }
                .frame(width: 50, height: 50)

                Text(title)
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
        }
    }
}

// MARK: - Recommendation Row

struct RecommendationRow: View {
    let icon: String
    let title: String
    let description: String
    let color: Color

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon)
                .font(.system(size: 14))
                .foregroundStyle(color)
                .frame(width: 24)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(description)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(3)
            }
        }
    }
}

// MARK: - Timeline Row

struct TimelineRow: View {
    let time: String
    let icon: String
    let color: Color
    let title: String
    let description: String

    var body: some View {
        HStack(spacing: 10) {
            VStack(spacing: 0) {
                Image(systemName: icon)
                    .font(.system(size: 10))
                    .foregroundStyle(color)
                Rectangle()
                    .fill(color.opacity(0.3))
                    .frame(width: 1, height: 20)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                Text(description)
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(2)
            }
            Spacer()
            Text(time)
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(XTheme.textTertiary)
        }
    }
}

// MARK: - Intelligence View (Knowledge Graph)

struct IntelligenceView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var graphNodes: [GraphNodeModel] = []
    @State private var selectedNode: GraphNodeModel?

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                header

                // Graph stats
                if let twin = runner.digitalTwin {
                    graphStats(twin)
                }

                // Entity browser
                entityBrowser

                // Selected node detail
                if let node = selectedNode {
                    nodeDetail(node)
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.digitalTwin == nil { runner.collectTwin() }
        }
    }

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "brain.fill")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 6)
            VStack(alignment: .leading, spacing: 4) {
                Text("Intelligence — Knowledge Graph")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("Every entity and relationship in your Mac, connected")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }

    private func graphStats(_ twin: DigitalTwin) -> some View {
        XCard {
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible()),
                GridItem(.flexible()),
                GridItem(.flexible()),
            ], spacing: 12) {
                StatBadge(icon: "gearshape.fill", label: "Processes", value: "\(twin.processes.total_processes)", color: XTheme.accent)
                StatBadge(icon: "app.fill", label: "Apps", value: "\(twin.applications.total_apps)", color: XTheme.accent)
                StatBadge(icon: "cubes.fill", label: "Frameworks", value: "\(twin.software_genome.frameworks.count)", color: XTheme.accent)
                StatBadge(icon: "doc.on.doc.fill", label: "Duplicates", value: "\(twin.filesystem.duplicate_clusters.count)", color: XTheme.accent)
            }
        }
    }

    private var entityBrowser: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Entities", icon: "circle.grid.hex.fill")

                if let twin = runner.digitalTwin {
                    // Top energy consumers
                    if !twin.energy.energy_consumers.isEmpty {
                        entitySection("Energy Consumers", icon: "bolt.fill", color: XTheme.danger) {
                            ForEach(Array(twin.energy.energy_consumers.prefix(10))) { consumer in
                                entityRow(consumer.name, "\(Int(consumer.energy_impact)) impact", "energy")
                            }
                        }
                    }

                    // Memory leaks
                    if !twin.memory.leak_candidates.isEmpty {
                        entitySection("Memory Leaks", icon: "drop.fill", color: XTheme.medium) {
                            ForEach(twin.memory.leak_candidates) { leak in
                                entityRow(leak.name, "Growing \(formatBytes(UInt64(leak.growth_rate_bytes_per_min))))/min", "leak")
                            }
                        }
                    }

                    // Suspicious apps
                    if !twin.applications.suspicious_apps.isEmpty {
                        entitySection("Suspicious Apps", icon: "exclamationmark.shield.fill", color: XTheme.danger) {
                            ForEach(twin.applications.suspicious_apps, id: \.self) { app in
                                entityRow(app, "Security concern", "suspicious")
                            }
                        }
                    }

                    // Duplicate clusters
                    if !twin.filesystem.duplicate_clusters.isEmpty {
                        entitySection("Duplicate Clusters", icon: "doc.on.doc.fill", color: XTheme.accent) {
                            ForEach(Array(twin.filesystem.duplicate_clusters.prefix(10))) { cluster in
                                entityRow(
                                    "\(cluster.files.count) duplicate files",
                                    formatBytes(cluster.total_size_bytes),
                                    "duplicate"
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    private func entitySection<C: View>(_ title: String, icon: String, color: Color, @ViewBuilder content: () -> C) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: icon)
                    .foregroundStyle(color)
                    .font(.system(size: 11))
                Text(title)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
            }
            content()
        }
    }

    private func entityRow(_ name: String, _ detail: String, _ category: String) -> some View {
        HStack(spacing: 8) {
            Circle()
                .fill(categoryColor(category))
                .frame(width: 6, height: 6)
            Text(name)
                .font(.system(size: 11))
                .foregroundStyle(XTheme.textPrimary)
            Spacer()
            Text(detail)
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(XTheme.textTertiary)
        }
        .padding(.vertical, 2)
    }

    private func categoryColor(_ category: String) -> Color {
        switch category {
        case "energy": return XTheme.danger
        case "leak": return XTheme.medium
        case "suspicious": return XTheme.danger
        case "duplicate": return XTheme.accent
        default: return XTheme.textTertiary
        }
    }

    private func nodeDetail(_ node: GraphNodeModel) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 8) {
                Text(node.label)
                    .font(.system(size: 14, weight: .bold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(node.type)
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(XTheme.textTertiary)
                if let health = node.healthScore {
                    HStack(spacing: 4) {
                        Text("Health:")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                        Text("\(Int(health * 100))%")
                            .font(.system(size: 11, weight: .bold))
                            .foregroundStyle(health > 0.7 ? XTheme.safe : health > 0.4 ? XTheme.medium : XTheme.danger)
                    }
                }
            }
        }
    }
}

// Simple graph node model for the GUI
struct GraphNodeModel: Identifiable {
    let id: String
    let type: String
    let label: String
    let healthScore: Double?
    let category: String?
}

// MARK: - Timeline View (Event Stream)

struct TimelineView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                header

                if let twin = runner.digitalTwin {
                    eventTimeline(twin)
                } else {
                    XCard {
                        VStack(spacing: 12) {
                            Image(systemName: "clock.fill")
                                .font(.system(size: 36, weight: .light))
                                .foregroundStyle(XTheme.accent)
                            Text("No timeline data yet")
                                .font(.system(size: 14))
                                .foregroundStyle(XTheme.textSecondary)
                            Button("Collect Twin Data") { runner.collectTwin() }
                                .buttonStyle(.borderedProminent)
                                .tint(XTheme.accent)
                        }
                        .frame(maxWidth: .infinity, minHeight: 200)
                    }
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.digitalTwin == nil { runner.collectTwin() }
        }
    }

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "clock.fill")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 6)
            VStack(alignment: .leading, spacing: 4) {
                Text("Timeline — Git for your Mac")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("Every state change, replayable")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }

    private func eventTimeline(_ twin: DigitalTwin) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 8) {
                XSectionHeader(title: "Event Stream", icon: "timeline", count: eventCount(twin))

                // Process anomalies
                ForEach(twin.processes.anomalies) { anomaly in
                    eventRow(
                        icon: anomaly.severity == "critical" ? "xmark.octagon.fill" : "exclamationmark.triangle.fill",
                        color: anomaly.severity == "critical" ? XTheme.danger : XTheme.medium,
                        title: anomaly.name,
                        description: anomaly.description,
                        category: anomaly.anomaly_type
                    )
                }

                // Memory leaks
                ForEach(twin.memory.leak_candidates) { leak in
                    eventRow(
                        icon: "drop.fill",
                        color: XTheme.medium,
                        title: "Memory Leak: \(leak.name)",
                        description: "Growing \(formatBytes(UInt64(leak.growth_rate_bytes_per_min))))/min for \(leak.duration_mins)m",
                        category: "memory_leak"
                    )
                }

                // Storage warnings
                if let forecast = twin.filesystem.exhaustion_forecast_days, forecast < 60 {
                    eventRow(
                        icon: "externaldrive.fill",
                        color: forecast < 7 ? XTheme.danger : XTheme.medium,
                        title: "Storage Warning",
                        description: "Disk will be full in ~\(forecast) days",
                        category: "storage"
                    )
                }

                // Suspicious apps
                ForEach(twin.applications.suspicious_apps, id: \.self) { app in
                    eventRow(
                        icon: "exclamationmark.shield.fill",
                        color: XTheme.danger,
                        title: "Suspicious App: \(app)",
                        description: "Application flagged for security review",
                        category: "security"
                    )
                }

                // Energy offenders
                ForEach(Array(twin.energy.energy_consumers.filter { $0.energy_impact > 50 }.prefix(5))) { consumer in
                    eventRow(
                        icon: "bolt.fill",
                        color: XTheme.danger,
                        title: "Energy Consumer: \(consumer.name)",
                        description: "Energy impact: \(Int(consumer.energy_impact))/100 (\(consumer.category))",
                        category: "energy"
                    )
                }

                if eventCount(twin) == 0 {
                    Text("No events detected — system is healthy")
                        .font(.system(size: 13))
                        .foregroundStyle(XTheme.safe)
                        .padding(.vertical, 8)
                }
            }
        }
    }

    private func eventCount(_ twin: DigitalTwin) -> Int {
        let anomalies = twin.processes.anomalies.count
        let leaks = twin.memory.leak_candidates.count
        let storage = (twin.filesystem.exhaustion_forecast_days.map { $0 < 60 ? 1 : 0 } ?? 0)
        let suspicious = twin.applications.suspicious_apps.count
        let energy = twin.energy.energy_consumers.filter { $0.energy_impact > 50 }.count
        return anomalies + leaks + storage + suspicious + energy
    }

    private func eventRow(icon: String, color: Color, title: String, description: String, category: String) -> some View {
        HStack(spacing: 10) {
            VStack(spacing: 0) {
                Image(systemName: icon)
                    .font(.system(size: 12))
                    .foregroundStyle(color)
                Rectangle()
                    .fill(color.opacity(0.2))
                    .frame(width: 1, height: 30)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                Text(description)
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(2)
            }
            Spacer()
            Text(category)
                .font(.system(size: 9, design: .monospaced))
                .foregroundStyle(XTheme.textTertiary)
        }
        .padding(.vertical, 4)
    }
}

// MARK: - Assistant View (Natural Language Chat)

struct AssistantView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var messages: [ChatMessage] = []
    @State private var inputText = ""
    @State private var isThinking = false

    private let suggestions = [
        "Why is my Mac slow?",
        "What changed today?",
        "What is using my storage?",
        "Can I delete Xcode DerivedData?",
        "Why is battery draining?",
        "What can I delete safely?",
    ]

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack(spacing: 12) {
                Image(systemName: "bubble.left.and.bubble.right.fill")
                    .font(.system(size: 20, weight: .bold))
                    .foregroundStyle(XTheme.accent)
                    .xGlow(XTheme.accent, radius: 4)
                VStack(alignment: .leading, spacing: 2) {
                    Text("Assistant")
                        .font(.system(size: 18, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.metallicGradient)
                    Text("Ask questions about your Mac — powered by the reasoning engine")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                }
                Spacer()
            }
            .padding(20)

            // Chat messages
            ScrollView {
                VStack(spacing: 12) {
                    if messages.isEmpty {
                        emptyState
                    } else {
                        ForEach(messages) { msg in
                            chatBubble(msg)
                        }
                    }
                    if isThinking {
                        thinkingBubble
                    }
                }
                .padding(20)
            }

            // Suggestions
            if messages.isEmpty {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(suggestions, id: \.self) { s in
                            Button(s) {
                                sendMessage(s)
                            }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                            .tint(XTheme.accent)
                        }
                    }
                    .padding(.horizontal, 20)
                    .padding(.bottom, 8)
                }
            }

            // Input
            inputBar
        }
        .background(XTheme.voidGradient)
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "brain.head.profile.fill")
                .font(.system(size: 48, weight: .light))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 8)
            Text("Ask me anything about your Mac")
                .font(.system(size: 16, weight: .semibold))
                .foregroundStyle(XTheme.textPrimary)
            Text("I can analyze your system, explain what's happening, predict problems, and recommend safe optimizations.")
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textSecondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, minHeight: 200)
    }

    private func chatBubble(_ msg: ChatMessage) -> some View {
        HStack {
            if msg.isUser { Spacer() }
            VStack(alignment: msg.isUser ? .trailing : .leading, spacing: 4) {
                Text(msg.content)
                    .font(.system(size: 13))
                    .foregroundStyle(msg.isUser ? XTheme.color.textPrimary : XTheme.color.textPrimary)
                    .padding(12)
                    .background(msg.isUser ? XTheme.accent : XTheme.cardBorder.opacity(0.5))
                    .cornerRadius(12)
                if !msg.isUser, let confidence = msg.confidence {
                    Text("Confidence: \(Int(confidence * 100))%")
                        .font(.system(size: 9, design: .monospaced))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
            if !msg.isUser { Spacer() }
        }
    }

    private var thinkingBubble: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    ProgressView()
                        .scaleEffect(0.6)
                    Text("Analyzing your system...")
                        .font(.system(size: 13))
                        .foregroundStyle(XTheme.textSecondary)
                }
                .padding(12)
                .background(XTheme.cardBorder.opacity(0.5))
                .cornerRadius(12)
            }
            Spacer()
        }
    }

    private var inputBar: some View {
        HStack(spacing: 10) {
            TextField("Ask a question...", text: $inputText)
                .textFieldStyle(.plain)
                .font(.system(size: 13))
                .padding(10)
                .background(XTheme.cardBorder.opacity(0.5))
                .cornerRadius(8)
                .onSubmit { sendCurrentMessage() }

            Button(action: sendCurrentMessage) {
                Image(systemName: "arrow.up.circle.fill")
                    .font(.system(size: 22))
                    .foregroundStyle(XTheme.accent)
            }
            .buttonStyle(.plain)
            .disabled(inputText.isEmpty || isThinking)
        }
        .padding(20)
    }

    private func sendCurrentMessage() {
        guard !inputText.isEmpty else { return }
        sendMessage(inputText)
        inputText = ""
    }

    private func sendMessage(_ text: String) {
        messages.append(ChatMessage(id: UUID().uuidString, content: text, isUser: true, confidence: nil))
        isThinking = true

        Task {
            runner.askTwin(question: text)
            // Wait for response
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            await MainActor.run {
                isThinking = false
                let response = runner.twinReasoningResult?.answer ?? "I'm still analyzing your system. Please try again in a moment."
                let confidence = runner.twinReasoningResult?.confidence
                messages.append(ChatMessage(id: UUID().uuidString, content: response, isUser: false, confidence: confidence))
            }
        }
    }
}

struct ChatMessage: Identifiable {
    let id: String
    let content: String
    let isUser: Bool
    let confidence: Double?
}

// MARK: - Stat Badge (defined in FullScanView.swift, reused here)

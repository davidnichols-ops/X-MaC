import SwiftUI

// MARK: - Twin Dashboard View

struct TwinDashboardView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Header
                twinHeader

                if runner.twinLoading {
                    twinLoadingView
                } else if let twin = runner.digitalTwin {
                    twinContent(twin)
                } else if let err = runner.twinError {
                    twinErrorView(err)
                } else {
                    twinEmptyView
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }

    private var twinHeader: some View {
        HStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Digital Twin")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("A live computational model of your Mac")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
            Button(action: { runner.collectTwin() }) {
                Label("Refresh", systemImage: "arrow.clockwise")
                    .font(.system(size: 12, weight: .medium))
            }
            .buttonStyle(.bordered)
            .tint(XTheme.accent)
        }
    }

    private var twinLoadingView: some View {
        VStack(spacing: 16) {
            ProgressView()
                .scaleEffect(1.5)
            Text("Collecting Digital Twin snapshot...")
                .font(.system(size: 14))
                .foregroundStyle(XTheme.textSecondary)
            Text("This scans hardware, software, filesystem, processes, memory, energy, and apps")
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity, minHeight: 300)
    }

    private func twinErrorView(_ err: String) -> some View {
        XCard {
            VStack(spacing: 8) {
                Image(systemName: "xmark.octagon.fill")
                    .font(.system(size: 32))
                    .foregroundStyle(XTheme.danger)
                Text("Failed to collect Digital Twin")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(err)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
                Button("Retry") { runner.collectTwin() }
                    .buttonStyle(.bordered)
                    .tint(XTheme.accent)
            }
            .frame(maxWidth: .infinity)
        }
    }

    private var twinEmptyView: some View {
        VStack(spacing: 16) {
            Image(systemName: "brain")
                .font(.system(size: 48, weight: .light))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 10)
            Text("Digital Twin Ready")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(XTheme.textPrimary)
            Text("Click Refresh to collect a full system snapshot")
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textSecondary)
            Button("Collect Now") { runner.collectTwin() }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
        }
        .frame(maxWidth: .infinity, minHeight: 300)
    }

    private func twinContent(_ twin: DigitalTwin) -> some View {
        VStack(spacing: 16) {
            // Health & Trust scores
            HStack(spacing: 16) {
                twinScoreCard(
                    title: "System Health",
                    score: twin.health_score,
                    icon: "heart.fill",
                    color: healthColor(twin.health_score)
                )
                twinScoreCard(
                    title: "Trust Score",
                    score: twin.trust_score * 100,
                    icon: "checkmark.shield.fill",
                    color: XTheme.accent
                )
                twinScoreCard(
                    title: "Total Processes",
                    score: Double(twin.processes.total_processes),
                    icon: "chart.bar.fill",
                    color: XTheme.teal,
                    isInteger: true
                )
                twinScoreCard(
                    title: "Total Apps",
                    score: Double(twin.applications.total_apps),
                    icon: "app.fill",
                    color: XTheme.teal,
                    isInteger: true
                )
            }

            // Quick navigation to sub-views
            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {
                twinNavCard(icon: "cpu.fill", title: "Hardware", subtitle: twin.hardware.model_identifier, destination: .twinHardware)
                twinNavCard(icon: "shippingbox.fill", title: "Software Genome", subtitle: "\(twin.software_genome.total_components) components", destination: .twinSoftware)
                twinNavCard(icon: "folder.fill.badge.gearshape", title: "Filesystem", subtitle: "\(formatBytes(twin.filesystem.total_size_bytes)) used", destination: .twinFilesystem)
                twinNavCard(icon: "chart.bar.fill", title: "Processes", subtitle: "\(twin.processes.total_processes) running", destination: .twinProcesses)
                twinNavCard(icon: "memorychip.fill", title: "Memory", subtitle: "\(String(format: "%.0f%%", twin.memory.utilization * 100)) used", destination: .twinMemory)
                twinNavCard(icon: "bolt.fill", title: "Energy", subtitle: twin.energy.battery?.condition ?? "No battery", destination: .twinEnergy)
                twinNavCard(icon: "app.connected.to.app.below.fill", title: "App Intelligence", subtitle: "\(twin.applications.suspicious_apps.count) suspicious", destination: .twinApps)
                twinNavCard(icon: "lightbulb.fill", title: "Reasoning Engine", subtitle: "Ask, predict, simulate", destination: .twinReasoning)
            }

            // Anomalies preview
            if !twin.processes.anomalies.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Process Anomalies", icon: "exclamationmark.triangle.fill", count: twin.processes.anomalies.count)
                        ForEach(twin.processes.anomalies.prefix(5)) { anomaly in
                            HStack(spacing: 8) {
                                Image(systemName: severityIcon(anomaly.severity))
                                    .foregroundStyle(severityColor(anomaly.severity))
                                    .font(.system(size: 11))
                                Text(anomaly.name)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text(anomaly.description)
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                    .lineLimit(1)
                                Spacer()
                            }
                        }
                        if twin.processes.anomalies.count > 5 {
                            Button("View all \(twin.processes.anomalies.count)") { runner.openTwinProcesses() }
                                .buttonStyle(.borderless)
                                .font(.system(size: 11))
                                .tint(XTheme.accent)
                        }
                    }
                }
            }

            // Memory leaks preview
            if !twin.memory.leak_candidates.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Memory Leak Candidates", icon: "drop.fill", count: twin.memory.leak_candidates.count)
                        ForEach(twin.memory.leak_candidates.prefix(5)) { leak in
                            HStack(spacing: 8) {
                                Image(systemName: "drop.triangle.fill")
                                    .foregroundStyle(XTheme.danger)
                                    .font(.system(size: 11))
                                Text(leak.name)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text("PID \(leak.pid)")
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.textTertiary)
                                Spacer()
                                Text("\(formatBytes(Int(leak.growth_rate_bytes_per_min)))/min")
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.medium)
                            }
                        }
                    }
                }
            }

            // Suspicious apps preview
            if !twin.applications.suspicious_apps.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Suspicious Apps", icon: "exclamationmark.shield.fill", count: twin.applications.suspicious_apps.count)
                        ForEach(twin.applications.suspicious_apps.prefix(5), id: \.self) { app in
                            HStack(spacing: 8) {
                                Image(systemName: "exclamationmark.triangle.fill")
                                    .foregroundStyle(XTheme.high)
                                    .font(.system(size: 11))
                                Text(app)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Spacer()
                            }
                        }
                    }
                }
            }
        }
    }

    private func twinScoreCard(title: String, score: Double, icon: String, color: Color, isInteger: Bool = false) -> some View {
        XCard {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.system(size: 20))
                    .foregroundStyle(color)
                Text(isInteger ? "\(Int(score))" : String(format: "%.0f", score))
                    .font(.system(size: 28, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.textPrimary)
                Text(title)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
        }
    }

    private func twinNavCard(icon: String, title: String, subtitle: String, destination: XMacRunner.ScanMode) -> some View {
        Button {
            switch destination {
            case .twinHardware: runner.openTwinHardware()
            case .twinSoftware: runner.openTwinSoftware()
            case .twinFilesystem: runner.openTwinFilesystem()
            case .twinProcesses: runner.openTwinProcesses()
            case .twinMemory: runner.openTwinMemory()
            case .twinEnergy: runner.openTwinEnergy()
            case .twinApps: runner.openTwinApps()
            case .twinReasoning: runner.openTwinReasoning()
            default: break
            }
        } label: {
            XCard {
                HStack(spacing: 12) {
                    Image(systemName: icon)
                        .font(.system(size: 22))
                        .foregroundStyle(XTheme.accent)
                        .frame(width: 32)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                        Text(subtitle)
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
        }
        .buttonStyle(.plain)
    }

    private func healthColor(_ score: Double) -> Color {
        if score >= 80 { return XTheme.safe }
        if score >= 60 { return XTheme.medium }
        if score >= 40 { return XTheme.high }
        return XTheme.critical
    }

    private func severityIcon(_ severity: String) -> String {
        switch severity.lowercased() {
        case "critical": return "xmark.octagon.fill"
        case "high": return "exclamationmark.octagon.fill"
        case "medium": return "exclamationmark.triangle.fill"
        case "low": return "checkmark.circle.fill"
        default: return "info.circle.fill"
        }
    }

    private func severityColor(_ severity: String) -> Color {
        switch severity.lowercased() {
        case "critical": return XTheme.critical
        case "high": return XTheme.high
        case "medium": return XTheme.medium
        case "low": return XTheme.low
        default: return XTheme.info
        }
    }
}

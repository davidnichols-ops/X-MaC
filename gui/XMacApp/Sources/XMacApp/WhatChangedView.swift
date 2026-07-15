import Foundation
import SwiftUI

// MARK: - What Changed? Models
// Swift Codable models matching the Rust `ChangeReport` JSON output from
// `xmac twin --action what-changed --since <since> --format json`.

struct WhatChangedReport: Codable {
    let start_ms: Int
    let end_ms: Int
    let total_events: Int
    let new_applications: [AppChange]
    let removed_applications: [AppChange]
    let storage_growth: StorageGrowth
    let process_changes: ProcessChanges
    let major_file_changes: [FileChange]
    let configuration_changes: [ConfigChange]
    let security_alerts: [SecurityAlert]
    let timeline: [TimelineEntry]
}

struct AppChange: Codable, Identifiable, Hashable {
    let name: String
    let bundle_id: String?
    let timestamp_ms: Int
    let source: String

    var id: String { "\(name)-\(timestamp_ms)" }
}

struct StorageGrowth: Codable, Hashable {
    let bytes_added: UInt64
    let bytes_removed: UInt64
    let net_change: Int
    let files_created: UInt64
    let files_deleted: UInt64
}

struct ProcessChanges: Codable, Hashable {
    let new_processes: UInt64
    let terminated_processes: UInt64
    let anomalies: UInt64
    let top_cpu_consumers: [String]
    let top_memory_consumers: [String]
}

struct FileChange: Codable, Identifiable, Hashable {
    let path: String
    let size_bytes: UInt64
    let change_type: String
    let timestamp_ms: Int
    let entity_id: String?

    var id: String { "\(path)-\(timestamp_ms)" }
}

struct ConfigChange: Codable, Identifiable, Hashable {
    let description: String
    let timestamp_ms: Int
    let source: String

    var id: String { "\(description)-\(timestamp_ms)" }
}

struct SecurityAlert: Codable, Identifiable, Hashable {
    let description: String
    let severity: String
    let timestamp_ms: Int

    var id: String { "\(description)-\(timestamp_ms)" }
}

struct TimelineEntry: Codable, Identifiable, Hashable {
    let timestamp_ms: Int
    let event_type: String
    let description: String
    let severity: String

    var id: String { "\(event_type)-\(timestamp_ms)-\(description)" }
}

// MARK: - What Changed? View

struct WhatChangedView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var sinceOption: SinceOption = .last24h

    enum SinceOption: String, CaseIterable, Identifiable {
        case last1h = "1h"
        case last6h = "6h"
        case last24h = "24h"
        case last7d = "7d"
        case last30d = "30d"

        var id: String { rawValue }

        var label: String {
            switch self {
            case .last1h: return "Last hour"
            case .last6h: return "Last 6 hours"
            case .last24h: return "Last 24 hours"
            case .last7d: return "Last 7 days"
            case .last30d: return "Last 30 days"
            }
        }
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                header

                if runner.whatChangedLoading {
                    loadingView
                } else if let report = runner.whatChanged {
                    if report.total_events == 0 {
                        noChangesView
                    } else {
                        reportContent(report)
                    }
                } else if let err = runner.whatChangedError {
                    errorView(err)
                } else {
                    placeholderView
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.whatChanged == nil && !runner.whatChangedLoading {
                runner.queryWhatChanged(since: sinceOption.rawValue)
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "arrow.triangle.2.circlepath")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 6)
            VStack(alignment: .leading, spacing: 4) {
                Text("What Changed?")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("Temporal diff — see exactly what happened on your Mac")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()

            Picker("Time Range", selection: $sinceOption) {
                ForEach(SinceOption.allCases) { option in
                    Text(option.label).tag(option)
                }
            }
            .pickerStyle(.menu)
            .tint(XTheme.accent)
            .frame(width: 160)
            .onChange(of: sinceOption) { _, newValue in
                runner.queryWhatChanged(since: newValue.rawValue)
            }

            Button {
                runner.queryWhatChanged(since: sinceOption.rawValue)
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 12, weight: .medium))
                    Text("Refresh")
                        .font(.system(size: 12, weight: .medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 7)
                .background(XTheme.accent.opacity(0.15))
                .foregroundStyle(XTheme.accent)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(XTheme.accent.opacity(0.3), lineWidth: 1)
                )
            }
            .buttonStyle(.plain)
            .disabled(runner.whatChangedLoading)
        }
    }

    // MARK: - Loading

    private var loadingView: some View {
        XCard {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.2)
                    .tint(XTheme.accent)
                Text("Analyzing changes...")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
                Text("Querying event log for the last \(sinceOption.label.lowercased())")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textTertiary)
            }
            .frame(maxWidth: .infinity, minHeight: 200)
        }
    }

    // MARK: - No Changes

    private var noChangesView: some View {
        XCard {
            VStack(spacing: 16) {
                Image(systemName: "checkmark.seal.fill")
                    .font(.system(size: 48, weight: .light))
                    .foregroundStyle(XTheme.safe)
                    .xGlow(XTheme.safe, radius: 8)
                Text("No changes detected")
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text("Your Mac has been stable during the selected period.")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity, minHeight: 200)
        }
    }

    // MARK: - Error

    private func errorView(_ message: String) -> some View {
        XCard {
            VStack(spacing: 12) {
                Image(systemName: "xmark.octagon.fill")
                    .font(.system(size: 36, weight: .light))
                    .foregroundStyle(XTheme.danger)
                Text("Failed to load changes")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(message)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
                    .multilineTextAlignment(.center)
                Button("Try Again") {
                    runner.queryWhatChanged(since: sinceOption.rawValue)
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
            }
            .frame(maxWidth: .infinity, minHeight: 180)
        }
    }

    // MARK: - Placeholder

    private var placeholderView: some View {
        XCard {
            VStack(spacing: 12) {
                Image(systemName: "arrow.triangle.2.circlepath")
                    .font(.system(size: 36, weight: .light))
                    .foregroundStyle(XTheme.accent)
                Text("No change data yet")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
                Button("Query Changes") {
                    runner.queryWhatChanged(since: sinceOption.rawValue)
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
            }
            .frame(maxWidth: .infinity, minHeight: 200)
        }
    }

    // MARK: - Report Content

    private func reportContent(_ report: WhatChangedReport) -> some View {
        VStack(spacing: 16) {
            // Summary card
            summaryCard(report)

            // New applications
            if !report.new_applications.isEmpty {
                appChangesSection(
                    title: "New Applications",
                    icon: "app.badge.checkmark",
                    changes: report.new_applications,
                    color: XTheme.safe
                )
            }

            // Removed applications
            if !report.removed_applications.isEmpty {
                appChangesSection(
                    title: "Removed Applications",
                    icon: "app.badge.minus",
                    changes: report.removed_applications,
                    color: XTheme.danger
                )
            }

            // Storage growth
            storageGrowthSection(report.storage_growth)

            // Process changes
            if report.process_changes.new_processes > 0
                || report.process_changes.terminated_processes > 0
                || report.process_changes.anomalies > 0
            {
                processChangesSection(report.process_changes)
            }

            // Major file changes
            if !report.major_file_changes.isEmpty {
                majorFileChangesSection(report.major_file_changes)
            }

            // Security alerts
            if !report.security_alerts.isEmpty {
                securityAlertsSection(report.security_alerts)
            }

            // Configuration changes
            if !report.configuration_changes.isEmpty {
                configChangesSection(report.configuration_changes)
            }

            // Timeline
            if !report.timeline.isEmpty {
                timelineSection(report.timeline)
            }
        }
    }

    // MARK: - Summary Card

    private func summaryCard(_ report: WhatChangedReport) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 8) {
                    Image(systemName: "chart.bar.doc.horizontal")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.accent)
                        .xGlow(XTheme.accent, radius: 4)
                    Text("Summary")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                    Spacer()
                    Text("\(report.total_events) total events")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(XTheme.accent)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(XTheme.accent.opacity(0.12))
                        .clipShape(Capsule())
                }

                Divider().background(XTheme.cardBorder)

                HStack(spacing: 24) {
                    summaryStat(
                        label: "New Apps",
                        value: "\(report.new_applications.count)",
                        icon: "app.fill",
                        color: XTheme.safe
                    )
                    summaryStat(
                        label: "Removed",
                        value: "\(report.removed_applications.count)",
                        icon: "app.dashed",
                        color: XTheme.danger
                    )
                    summaryStat(
                        label: "File Changes",
                        value: "\(report.major_file_changes.count)",
                        icon: "doc.fill",
                        color: XTheme.medium
                    )
                    summaryStat(
                        label: "Alerts",
                        value: "\(report.security_alerts.count)",
                        icon: "exclamationmark.shield.fill",
                        color: report.security_alerts.isEmpty ? XTheme.textTertiary : XTheme.high
                    )
                }

                HStack(spacing: 6) {
                    Image(systemName: "calendar")
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textTertiary)
                    Text("Period: \(formatTimestamp(report.start_ms)) → \(formatTimestamp(report.end_ms))")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
        }
    }

    private func summaryStat(label: String, value: String, icon: String, color: Color) -> some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .font(.system(size: 16))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 18, weight: .bold, design: .rounded))
                .foregroundStyle(XTheme.textPrimary)
            Text(label)
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - App Changes Section

    private func appChangesSection(title: String, icon: String, changes: [AppChange], color: Color) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: title, icon: icon, count: changes.count)

                ForEach(changes) { app in
                    HStack(spacing: 10) {
                        Image(systemName: color == XTheme.safe ? "plus.circle.fill" : "minus.circle.fill")
                            .font(.system(size: 14))
                            .foregroundStyle(color)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(app.name)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                            HStack(spacing: 8) {
                                if let bid = app.bundle_id, !bid.isEmpty {
                                    Text(bid)
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.textTertiary)
                                }
                                Text(app.source)
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                        }
                        Spacer()
                        Text(formatTimestamp(app.timestamp_ms))
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    .padding(.vertical, 4)
                }
            }
        }
    }

    // MARK: - Storage Growth Section

    private func storageGrowthSection(_ sg: StorageGrowth) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Storage Growth", icon: "internaldrive")

                HStack(spacing: 16) {
                    storageStat(
                        label: "Added",
                        value: formatBytes(Int(sg.bytes_added)),
                        icon: "arrow.up.circle.fill",
                        color: XTheme.safe
                    )
                    storageStat(
                        label: "Removed",
                        value: formatBytes(Int(sg.bytes_removed)),
                        icon: "arrow.down.circle.fill",
                        color: XTheme.danger
                    )
                    storageStat(
                        label: "Net Change",
                        value: formatNetChange(sg.net_change),
                        icon: "scalemass.fill",
                        color: sg.net_change >= 0 ? XTheme.medium : XTheme.safe
                    )
                }

                Divider().background(XTheme.cardBorder)

                HStack(spacing: 16) {
                    storageStat(
                        label: "Files Created",
                        value: "\(sg.files_created)",
                        icon: "doc.badge.plus",
                        color: XTheme.safe
                    )
                    storageStat(
                        label: "Files Deleted",
                        value: "\(sg.files_deleted)",
                        icon: "doc.badge.minus",
                        color: XTheme.danger
                    )
                    Spacer()
                }
            }
        }
    }

    private func storageStat(label: String, value: String, icon: String, color: Color) -> some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 18))
                .foregroundStyle(color)
            VStack(alignment: .leading, spacing: 2) {
                Text(value)
                    .font(.system(size: 14, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.textPrimary)
                Text(label)
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textTertiary)
            }
        }
    }

    // MARK: - Process Changes Section

    private func processChangesSection(_ pc: ProcessChanges) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Process Changes", icon: "gearshape.2.fill")

                HStack(spacing: 16) {
                    processStat(label: "Launched", value: "\(pc.new_processes)", color: XTheme.safe)
                    processStat(label: "Terminated", value: "\(pc.terminated_processes)", color: XTheme.danger)
                    processStat(label: "Anomalies", value: "\(pc.anomalies)", color: pc.anomalies > 0 ? XTheme.high : XTheme.textTertiary)
                }

                if !pc.top_cpu_consumers.isEmpty {
                    Divider().background(XTheme.cardBorder)
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Top CPU Consumers")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(XTheme.textSecondary)
                        ForEach(pc.top_cpu_consumers, id: \.self) { name in
                            HStack(spacing: 6) {
                                Image(systemName: "flame.fill")
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.high)
                                Text(name)
                                    .font(.system(size: 12, design: .monospaced))
                                    .foregroundStyle(XTheme.textPrimary)
                            }
                        }
                    }
                }

                if !pc.top_memory_consumers.isEmpty {
                    Divider().background(XTheme.cardBorder)
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Top Memory Consumers")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(XTheme.textSecondary)
                        ForEach(pc.top_memory_consumers, id: \.self) { name in
                            HStack(spacing: 6) {
                                Image(systemName: "memorychip")
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.accent)
                                Text(name)
                                    .font(.system(size: 12, design: .monospaced))
                                    .foregroundStyle(XTheme.textPrimary)
                            }
                        }
                    }
                }
            }
        }
    }

    private func processStat(label: String, value: String, color: Color) -> some View {
        VStack(spacing: 4) {
            Text(value)
                .font(.system(size: 16, weight: .bold, design: .rounded))
                .foregroundStyle(color)
            Text(label)
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Major File Changes Section

    private func majorFileChangesSection(_ changes: [FileChange]) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Major File Changes", icon: "doc.fill", count: changes.count)

                ForEach(changes) { fc in
                    HStack(spacing: 10) {
                        Image(systemName: fileChangeIcon(fc.change_type))
                            .font(.system(size: 14))
                            .foregroundStyle(fileChangeColor(fc.change_type))
                        VStack(alignment: .leading, spacing: 2) {
                            Text((fc.path as NSString).lastPathComponent)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Text(fc.path)
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundStyle(XTheme.textTertiary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        Spacer()
                        VStack(alignment: .trailing, spacing: 2) {
                            Text(formatBytes(Int(fc.size_bytes)))
                                .font(.system(size: 12, weight: .semibold, design: .rounded))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(formatTimestamp(fc.timestamp_ms))
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                    }
                    .padding(.vertical, 4)
                }
            }
        }
    }

    private func fileChangeIcon(_ type: String) -> String {
        switch type {
        case "created": return "plus.circle.fill"
        case "deleted": return "minus.circle.fill"
        case "grown": return "arrow.up.right.circle.fill"
        default: return "doc.fill"
        }
    }

    private func fileChangeColor(_ type: String) -> Color {
        switch type {
        case "created": return XTheme.safe
        case "deleted": return XTheme.danger
        case "grown": return XTheme.medium
        default: return XTheme.textTertiary
        }
    }

    // MARK: - Security Alerts Section

    private func securityAlertsSection(_ alerts: [SecurityAlert]) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Security Alerts", icon: "exclamationmark.shield.fill", count: alerts.count)

                ForEach(alerts) { alert in
                    HStack(spacing: 10) {
                        Image(systemName: severityIcon(alert.severity))
                            .font(.system(size: 14))
                            .foregroundStyle(severityColor(alert.severity))
                        VStack(alignment: .leading, spacing: 2) {
                            Text(alert.description)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(alert.severity.capitalized)
                                .font(.system(size: 10, weight: .medium))
                                .foregroundStyle(severityColor(alert.severity))
                        }
                        Spacer()
                        Text(formatTimestamp(alert.timestamp_ms))
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    .padding(.vertical, 4)
                }
            }
        }
    }

    // MARK: - Config Changes Section

    private func configChangesSection(_ changes: [ConfigChange]) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Configuration & Optimization Changes", icon: "gearshape.fill", count: changes.count)

                ForEach(changes) { cc in
                    HStack(spacing: 10) {
                        Image(systemName: "gearshape.2.fill")
                            .font(.system(size: 14))
                            .foregroundStyle(XTheme.medium)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(cc.description)
                                .font(.system(size: 13, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(cc.source)
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                        Spacer()
                        Text(formatTimestamp(cc.timestamp_ms))
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    .padding(.vertical, 4)
                }
            }
        }
    }

    // MARK: - Timeline Section

    private func timelineSection(_ entries: [TimelineEntry]) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Event Timeline", icon: "timeline", count: entries.count)

                ForEach(entries) { entry in
                    HStack(spacing: 10) {
                        // Vertical timeline dot
                        Circle()
                            .fill(severityColor(entry.severity))
                            .frame(width: 8, height: 8)
                            .xGlow(severityColor(entry.severity), radius: 3)

                        VStack(alignment: .leading, spacing: 2) {
                            Text(entry.description.isEmpty ? entry.event_type : entry.description)
                                .font(.system(size: 12, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                                .lineLimit(2)
                            HStack(spacing: 6) {
                                Text(entry.event_type)
                                    .font(.system(size: 10, design: .monospaced))
                                    .foregroundStyle(XTheme.textTertiary)
                                Text("·")
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                                Text(formatTimestamp(entry.timestamp_ms))
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                        }
                        Spacer()
                        Text(entry.severity.capitalized)
                            .font(.system(size: 9, weight: .medium))
                            .foregroundStyle(severityColor(entry.severity))
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(severityColor(entry.severity).opacity(0.12))
                            .clipShape(Capsule())
                    }
                    .padding(.vertical, 3)
                }
            }
        }
    }

    // MARK: - Helpers

    private func severityColor(_ severity: String) -> Color {
        switch severity.lowercased() {
        case "info": return XTheme.info
        case "low": return XTheme.low
        case "warning", "medium": return XTheme.medium
        case "error", "high": return XTheme.high
        case "critical": return XTheme.critical
        default: return XTheme.textTertiary
        }
    }

    private func severityIcon(_ severity: String) -> String {
        switch severity.lowercased() {
        case "info": return "info.circle.fill"
        case "low": return "checkmark.circle.fill"
        case "warning", "medium": return "exclamationmark.triangle.fill"
        case "error", "high": return "exclamationmark.octagon.fill"
        case "critical": return "xmark.octagon.fill"
        default: return "info.circle.fill"
        }
    }

    /// Format a millisecond timestamp as a relative string ("2 hours ago").
    private func formatTimestamp(_ ms: Int) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(ms) / 1000.0)
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .short
        return formatter.localizedString(for: date, relativeTo: Date())
    }

    /// Format a signed net change with + / - prefix.
    private func formatNetChange(_ bytes: Int) -> String {
        let prefix = bytes >= 0 ? "+" : "-"
        return "\(prefix)\(formatBytes(abs(bytes)))"
    }
}

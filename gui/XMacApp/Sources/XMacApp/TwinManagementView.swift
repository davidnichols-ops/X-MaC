import Foundation
import SwiftUI

// MARK: - Twin Management View

struct TwinManagementView: View {
    @EnvironmentObject var runner: XMacRunner
    @EnvironmentObject var router: AppRouter
    @State private var durationOption: DurationOption = .thirtySeconds
    @State private var showingInitConfirmation = false
    @State private var showingCompactConfirmation = false

    enum DurationOption: String, CaseIterable, Identifiable {
        case tenSeconds = "10s"
        case thirtySeconds = "30s"
        case oneMinute = "1m"
        case fiveMinutes = "5m"
        case thirtyMinutes = "30m"

        var id: String { rawValue }

        var label: String {
            switch self {
            case .tenSeconds: return "10 seconds"
            case .thirtySeconds: return "30 seconds"
            case .oneMinute: return "1 minute"
            case .fiveMinutes: return "5 minutes"
            case .thirtyMinutes: return "30 minutes"
            }
        }
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                header

                databaseSection
                observerSection
                whatChangedSection
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "externaldrive.connected.to.line.below")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 6)
            VStack(alignment: .leading, spacing: 4) {
                Text("Twin Management")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("Database lifecycle, observers, and temporal diff access")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }

    // MARK: - Database Management

    private var databaseSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Database Management", icon: "cylinder.split.1x2.fill")

                Text("Initialize or compact the digital twin database. The twin stores telemetry snapshots used for temporal diffs and predictions.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)

                HStack(spacing: 12) {
                    Button {
                        showingInitConfirmation = true
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "plus.cylinder")
                                .font(.system(size: 12, weight: .medium))
                            Text("Init DB")
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(XTheme.accent.opacity(0.15))
                        .foregroundStyle(XTheme.accent)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(XTheme.accent.opacity(0.3), lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .confirmationDialog(
                        "Initialize digital twin database?",
                        isPresented: $showingInitConfirmation,
                        titleVisibility: .visible
                    ) {
                        Button("Initialize", role: .destructive) {
                            runner.initTwinDb()
                        }
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("This will create a new twin database. If one already exists, it may be replaced.")
                    }

                    Button {
                        showingCompactConfirmation = true
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "arrow.compress.cylinder")
                                .font(.system(size: 12, weight: .medium))
                            Text("Compact DB")
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(XTheme.bgTertiary)
                        .foregroundStyle(XTheme.textPrimary)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(XTheme.cardBorder, lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .confirmationDialog(
                        "Compact digital twin database?",
                        isPresented: $showingCompactConfirmation,
                        titleVisibility: .visible
                    ) {
                        Button("Compact", role: .destructive) {
                            runner.compactTwinDb()
                        }
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("This will reclaim unused space in the twin database. The operation may take a few moments.")
                    }

                    Spacer()

                    if runner.twinDbInitialized {
                        HStack(spacing: 6) {
                            Image(systemName: "checkmark.circle.fill")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.safe)
                            Text("Initialized")
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(XTheme.safe)
                        }
                    }
                }

                if let message = runner.twinDbMessage {
                    HStack(spacing: 8) {
                        Image(systemName: "info.circle.fill")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textTertiary)
                        Text(message)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(XTheme.textSecondary)
                            .lineLimit(3)
                    }
                    .padding(.top, 4)
                }
            }
        }
    }

    // MARK: - Observer Controls

    private var observerSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Observer Controls", icon: "eye.fill")

                Text("Run observers to capture live telemetry into the twin database for a fixed duration.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)

                HStack(spacing: 12) {
                    Picker("Duration", selection: $durationOption) {
                        ForEach(DurationOption.allCases) { option in
                            Text(option.label).tag(option)
                        }
                    }
                    .pickerStyle(.menu)
                    .tint(XTheme.accent)
                    .frame(width: 160)

                    Button {
                        runner.runObservers(duration: durationOption.rawValue)
                    } label: {
                        HStack(spacing: 6) {
                            if runner.isObserving {
                                ProgressView()
                                    .scaleEffect(0.7)
                                    .tint(XTheme.accent)
                            } else {
                                Image(systemName: "play.fill")
                                    .font(.system(size: 12, weight: .medium))
                            }
                            Text("Start Observers")
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(XTheme.accent.opacity(0.15))
                        .foregroundStyle(XTheme.accent)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(XTheme.accent.opacity(0.3), lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(runner.isObserving)

                    Spacer()

                    if runner.isObserving {
                        HStack(spacing: 6) {
                            Circle()
                                .fill(XTheme.accent)
                                .frame(width: 8, height: 8)
                                .xGlow(XTheme.accent, radius: 4)
                            Text("Observing...")
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(XTheme.accent)
                        }
                    }
                }

                if let message = runner.observeMessage {
                    HStack(spacing: 8) {
                        Image(systemName: "info.circle.fill")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textTertiary)
                        Text(message)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(XTheme.textSecondary)
                            .lineLimit(3)
                    }
                    .padding(.top, 4)
                }
            }
        }
    }

    // MARK: - What Changed Quick Access

    private var whatChangedSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "What Changed? Quick Access", icon: "arrow.triangle.2.circlepath")

                Text("Jump to the temporal diff view to see exactly what changed on your Mac over a selected time range.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)

                Button {
                    router.navigate(to: .historyWhatChanged)
                } label: {
                    HStack(spacing: 6) {
                        Image(systemName: "arrow.triangle.2.circlepath")
                            .font(.system(size: 12, weight: .medium))
                        Text("Open What Changed?")
                            .font(.system(size: 12, weight: .medium))
                    }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
                    .background(XTheme.accent.opacity(0.15))
                    .foregroundStyle(XTheme.accent)
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(XTheme.accent.opacity(0.3), lineWidth: 1)
                    )
                }
                .buttonStyle(.plain)
            }
        }
    }
}

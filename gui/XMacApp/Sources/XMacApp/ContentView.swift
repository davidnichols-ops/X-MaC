import SwiftUI

struct ContentView: View {
    @EnvironmentObject var runner: XMacRunner
    @EnvironmentObject var settings: AppSettings
    @AppStorage("hasCompletedOnboarding") private var hasCompletedOnboarding: Bool = false

    var body: some View {
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 220, ideal: 240)
        } detail: {
            switch runner.scanMode {
            case .dashboard, .idle:
                DashboardView()
            case .full:
                FullScanView()
            case .clean:
                CleanView()
            case .maintain:
                MaintainView()
            case .ramBoost:
                RamBoostView()
            case .disk:
                DiskView()
            case .neural:
                NeuralScanView()
            case .apps:
                AppInventoryView()
            case .settings:
                SettingsView()
            case .history:
                ScanHistoryView()
            case .automation:
                AutomationView()
            }
        }
        .frame(minWidth: 1100, minHeight: 720)
        .background(XTheme.voidGradient)
        .overlay(alignment: .top) {
            ActivityBannerView()
                .padding(.horizontal, 16)
                .padding(.top, 8)
                .transition(.move(edge: .top).combined(with: .opacity))
                .animation(.easeInOut(duration: 0.3), value: runner.showActivityBanner)
        }
        .overlay(
            Group {
                if !hasCompletedOnboarding {
                    OnboardingView(
                        onComplete: { hasCompletedOnboarding = true },
                        onQuickScan: { hasCompletedOnboarding = true; runner.startCleanScan() }
                    )
                    .transition(.opacity)
                    .animation(.easeInOut(duration: 0.4), value: hasCompletedOnboarding)
                }
            }
        )
    }
}

// MARK: - Activity Banner

struct ActivityBannerView: View {
    @EnvironmentObject var runner: XMacRunner

    private var activity: ActivityLogEntry? {
        runner.lastActivity
    }

    private var showBanner: Bool {
        runner.showActivityBanner && activity != nil
    }

    var body: some View {
        if showBanner, let entry = activity {
            HStack(spacing: 10) {
                Image(systemName: iconForStatus(entry.status))
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(colorForStatus(entry.status))

                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.operation)
                        .font(.system(size: 12, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                    Text(entry.message)
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                        .lineLimit(2)
                }

                Spacer()

                if let duration = entry.durationMs {
                    Text(String(format: "%.0fms", duration))
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(XTheme.textTertiary)
                }

                Button {
                    runner.dismissActivityBanner()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(XTheme.textTertiary)
                }
                .buttonStyle(.plain)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            .background(
                RoundedRectangle(cornerRadius: 10)
                    .fill(XTheme.cardBg)
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(colorForStatus(entry.status).opacity(0.3), lineWidth: 1)
                    )
                    .shadow(color: colorForStatus(entry.status).opacity(0.15), radius: 8)
            )
        }
    }

    private func iconForStatus(_ status: ActivityStatus) -> String {
        switch status {
        case .started: return "arrow.clockwise.circle.fill"
        case .success: return "checkmark.circle.fill"
        case .warning: return "exclamationmark.triangle.fill"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private func colorForStatus(_ status: ActivityStatus) -> Color {
        switch status {
        case .started: return XTheme.accent
        case .success: return XTheme.safe
        case .warning: return XTheme.medium
        case .failed: return XTheme.danger
        }
    }
}

struct SidebarView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        VStack(spacing: 4) {
            // Logo — metallic X with cyan neural glow
            HStack(spacing: 10) {
                Image(systemName: "cpu")
                    .font(.system(size: 24, weight: .bold))
                    .foregroundStyle(XTheme.metallicGradient)
                    .xGlow(XTheme.accent, radius: 6)
                Text("X-MaC")
                    .font(.system(size: 18, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
            }
            .padding(.vertical, 16)

            Divider().background(XTheme.cardBorder)

            let scanning = runner.isScanning

            NavButton(icon: "rectangle.grid.2x2", label: "Overview", isActive: runner.scanMode == .dashboard || runner.scanMode == .idle, disabled: scanning) {
                runner.openDashboard()
            }
            NavButton(icon: "sparkles", label: "Smart Scan", isActive: runner.scanMode == .full, disabled: scanning) {
                runner.startFullScan()
            }
            NavButton(icon: "wand.and.stars", label: "Neural Scan", isActive: runner.scanMode == .neural, disabled: scanning) {
                runner.startNeuralScan()
            }
            NavButton(icon: "trash.circle", label: "Clean", isActive: runner.scanMode == .clean, disabled: scanning) {
                runner.startCleanScan()
            }
            NavButton(icon: "wrench.and.screwdriver", label: "Maintain", isActive: runner.scanMode == .maintain, disabled: scanning) {
                runner.startMaintainScan()
            }
            NavButton(icon: "bolt.fill", label: "RAM Boost", isActive: runner.scanMode == .ramBoost, disabled: scanning) {
                runner.openRamBoost()
            }
            NavButton(icon: "internaldrive", label: "Disk Usage", isActive: runner.scanMode == .disk, disabled: scanning) {
                runner.startDiskScan()
            }
            NavButton(icon: "app.badge", label: "Applications", isActive: runner.scanMode == .apps, disabled: scanning) {
                runner.openApps()
            }

            Spacer()

            if scanning {
                ScanProgressMini(phase: runner.scanPhase, progress: runner.scanProgress)
            } else if let activity = runner.lastActivity {
                ActivityMiniIndicator(activity: activity)
            }

            NavButton(icon: "clock.arrow.circlepath", label: "History", isActive: runner.scanMode == .history, disabled: scanning) {
                runner.openHistory()
            }
            NavButton(icon: "gearshape.2", label: "Automation", isActive: runner.scanMode == .automation, disabled: scanning) {
                runner.openAutomation()
            }
            NavButton(icon: "gearshape", label: "Settings", isActive: runner.scanMode == .settings, disabled: scanning) {
                runner.openSettings()
            }
        }
        .padding(.horizontal, 12)
        .background(XTheme.sidebarGradient)
    }
}

struct ScanProgressMini: View {
    @EnvironmentObject var runner: XMacRunner
    let phase: String
    let progress: Double

    var body: some View {
        VStack(spacing: 8) {
            HStack(spacing: 8) {
                ProgressView()
                    .scaleEffect(0.7)
                Text(phase)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(2)
                Spacer()
                Button("Stop") {
                    runner.stopScan()
                }
                .buttonStyle(.borderless)
                .controlSize(.mini)
                .tint(XTheme.danger)
            }
            ProgressView(value: progress)
                .tint(XTheme.accent)
                .scaleEffect(y: 0.6)
        }
        .padding(12)
        .background(XTheme.cardBg)
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .padding(.bottom, 8)
    }
}

struct ActivityMiniIndicator: View {
    let activity: ActivityLogEntry

    private var color: Color {
        switch activity.status {
        case .started: return XTheme.accent
        case .success: return XTheme.safe
        case .warning: return XTheme.medium
        case .failed: return XTheme.danger
        }
    }

    private var icon: String {
        switch activity.status {
        case .started: return "arrow.clockwise"
        case .success: return "checkmark.circle"
        case .warning: return "exclamationmark.triangle"
        case .failed: return "xmark.octagon"
        }
    }

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: icon)
                .font(.system(size: 10))
                .foregroundStyle(color)
            Text(activity.operation)
                .font(.system(size: 10, weight: .medium))
                .foregroundStyle(XTheme.textSecondary)
                .lineLimit(1)
            Spacer()
            Text(activity.message)
                .font(.system(size: 9))
                .foregroundStyle(XTheme.textTertiary)
                .lineLimit(1)
                .truncationMode(.tail)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 6)
        .background(XTheme.cardBg)
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(color.opacity(0.2), lineWidth: 1)
        )
        .padding(.bottom, 8)
    }
}

struct NavButton: View {
    let icon: String
    let label: String
    let isActive: Bool
    let disabled: Bool
    let action: () -> Void

    var body: some View {
        Button(action: disabled ? {} : action) {
            HStack(spacing: 10) {
                Image(systemName: icon)
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(foregroundColor)
                    .frame(width: 20)
                    .xGlow(isActive && !disabled ? XTheme.accent : .clear, radius: 3)
                Text(label)
                    .font(.system(size: 13, weight: isActive ? .semibold : .regular))
                    .foregroundStyle(foregroundColor)
                Spacer()
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(
                isActive && !disabled
                    ? LinearGradient(colors: [XTheme.accent.opacity(0.15), XTheme.accent.opacity(0.05)], startPoint: .leading, endPoint: .trailing)
                    : LinearGradient(colors: [.clear, .clear], startPoint: .leading, endPoint: .trailing)
            )
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .overlay(
                isActive && !disabled
                    ? RoundedRectangle(cornerRadius: 8)
                        .stroke(XTheme.accent.opacity(0.2), lineWidth: 1)
                    : nil
            )
        }
        .buttonStyle(.plain)
        .disabled(disabled)
        .opacity(disabled ? 0.5 : 1.0)
        .accessibilityLabel(label)
    }

    private var foregroundColor: Color {
        if disabled { return XTheme.textTertiary }
        return isActive ? XTheme.accent : XTheme.textSecondary
    }
}

struct WelcomeView: View {
    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "cpu")
                .font(.system(size: 64, weight: .light))
                .foregroundStyle(XTheme.metallicGradient)
                .xHeroGlow()

            Text("X-MaC")
                .font(.system(size: 32, weight: .bold, design: .rounded))
                .foregroundStyle(XTheme.metallicGradient)

            Text("macOS cleaner, optimizer, and system scanner")
                .font(.system(size: 14))
                .foregroundStyle(XTheme.textSecondary)

            Text("Select a scan mode from the sidebar to begin.")
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textTertiary)

            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(XTheme.voidGradient)
    }
}

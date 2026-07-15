import SwiftUI

// MARK: - ContentView
//
// Main app shell using the new 5-section navigation model.
// Routes NavigationDestination to the appropriate view.
// During migration, existing views are wrapped; they'll be rebuilt feature by feature.

struct ContentView: View {
    @EnvironmentObject var router: AppRouter
    @EnvironmentObject var runner: XMacRunner
    @EnvironmentObject var settings: AppSettings
    @AppStorage("hasCompletedOnboarding") private var hasCompletedOnboarding: Bool = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        NavigationSplitView {
            SidebarView()
                .navigationSplitViewColumnWidth(min: 220, ideal: 240)
        } detail: {
            detailView
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(XTheme.gradient.voidBackground)
                .id(router.selectedDestination) // Ensure clean state on navigation
        }
        .frame(minWidth: 1100, minHeight: 720)
        .overlay(alignment: .top) {
            ActivityBannerView()
                .padding(.horizontal, XTheme.spacing.lg)
                .padding(.top, XTheme.spacing.sm)
                .transition(.move(edge: .top).combined(with: .opacity))
                .animation(
                    XTheme.animation.resolved(reduceMotion: reduceMotion, duration: 0.3) ?? .easeInOut(duration: 0.3),
                    value: runner.showActivityBanner
                )
        }
        .overlay(
            Group {
                if !hasCompletedOnboarding {
                    OnboardingView(
                        onComplete: { hasCompletedOnboarding = true },
                        onQuickScan: {
                            hasCompletedOnboarding = true
                            router.navigate(to: .improveClean)
                            runner.startCleanScan()
                        }
                    )
                    .transition(.opacity)
                    .animation(
                        XTheme.animation.resolved(reduceMotion: reduceMotion, duration: 0.4) ?? .easeInOut(duration: 0.4),
                        value: hasCompletedOnboarding
                    )
                }
            }
        )
    }

    // MARK: - Detail View Routing

    @ViewBuilder
    private var detailView: some View {
        switch router.selectedDestination {
        case .home:
            HomeView()

        case .exploreSystem:
            systemDetailView

        case .exploreApplications:
            AppInventoryView()

        case .exploreFilesystem:
            filesystemDetailView

        case .exploreActivity:
            activityDetailView

        case .improveQuickScan:
            QuickScanView()

        case .improveClean:
            CleanView()

        case .improveNeuralScan:
            NeuralScanView()

        case .improveZenMode:
            ZenView()

        case .improveAIAdvisor:
            AdvisorView()

        case .historyWhatChanged:
            WhatChangedView()

        case .historyScanHistory:
            ScanHistoryView()

        case .controlAutomation:
            AutomationView()

        case .controlSafety:
            SafetyView()

        case .controlDiagnostics:
            DiagnosticsView()

        case .controlSettings:
            SettingsView()
        }
    }

    // MARK: - Sub-destination routing

    @ViewBuilder
    private var systemDetailView: some View {
        switch router.selectedSubDestination?.id {
        case "hardware":
            TwinHardwareView()
        case "software":
            TwinSoftwareView()
        case "memory":
            TwinMemoryView()
        case "energy":
            TwinEnergyView()
        default:
            TwinDashboardView()
        }
    }

    @ViewBuilder
    private var filesystemDetailView: some View {
        switch router.selectedSubDestination?.id {
        case "disk":
            DiskView()
        case "clean":
            CleanView()
        case "integrity":
            DepthView()
        case "safety":
            SafetyView()
        default:
            DiskView()
        }
    }

    @ViewBuilder
    private var activityDetailView: some View {
        switch router.selectedSubDestination?.id {
        case "processes":
            TwinProcessView()
        case "memory":
            TwinMemoryView()
        case "ramboost":
            RamBoostView()
        case "energy":
            TwinEnergyView()
        default:
            TwinProcessView()
        }
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
            HStack(spacing: XTheme.spacing.sm + 2) {
                Image(systemName: iconForStatus(entry.status))
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(colorForStatus(entry.status))

                VStack(alignment: .leading, spacing: 2) {
                    Text(entry.operation)
                        .font(XTheme.typography.bodySemibold)
                        .foregroundStyle(XTheme.color.textPrimary)
                    Text(entry.message)
                        .font(XTheme.typography.caption)
                        .foregroundStyle(XTheme.color.textSecondary)
                        .lineLimit(2)
                }

                Spacer()

                if let duration = entry.durationMs {
                    Text(String(format: "%.0fms", duration))
                        .font(XTheme.typography.monoSmall)
                        .foregroundStyle(XTheme.color.textTertiary)
                }

                Button {
                    runner.dismissActivityBanner()
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(XTheme.color.textTertiary)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Dismiss banner")
            }
            .padding(.horizontal, XTheme.spacing.md + 2)
            .padding(.vertical, XTheme.spacing.sm + 2)
            .background(XTheme.color.backgroundTertiary)
            .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.lg))
            .overlay(
                RoundedRectangle(cornerRadius: XTheme.radius.lg)
                    .stroke(colorForStatus(entry.status).opacity(0.2), lineWidth: XTheme.border.standard)
            )
            .shadow(color: colorForStatus(entry.status).opacity(0.15), radius: 8)
            .accessibilityElement(children: .combine)
            .accessibilityLabel("\(entry.operation): \(entry.message)")
        }
    }

    private func iconForStatus(_ status: ActivityStatus) -> String {
        switch status {
        case .started: return "circle.dotted"
        case .success: return "checkmark.circle.fill"
        case .warning: return "exclamationmark.triangle.fill"
        case .failed: return "xmark.octagon.fill"
        }
    }

    private func colorForStatus(_ status: ActivityStatus) -> Color {
        switch status {
        case .started: return XTheme.color.accent
        case .success: return XTheme.color.statusSafe
        case .warning: return XTheme.color.statusWarning
        case .failed: return XTheme.color.statusDanger
        }
    }
}

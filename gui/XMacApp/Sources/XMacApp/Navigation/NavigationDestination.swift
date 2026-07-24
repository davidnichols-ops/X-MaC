import SwiftUI

// MARK: - NavigationDestination
//
// 6-section navigation model with a dedicated Digital Twin control panel.
// The Digital Twin section surfaces all 10 twin dimensions as first-class
// destinations — no longer buried under Explore > System.

enum NavigationSection: String, CaseIterable, Identifiable {
    case home
    case twin
    case explore
    case improve
    case history
    case control

    var id: String { rawValue }

    var label: String {
        switch self {
        case .home: return "Home"
        case .twin: return "Digital Twin"
        case .explore: return "Explore"
        case .improve: return "Improve"
        case .history: return "History"
        case .control: return "Control"
        }
    }

    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .twin: return "circle.hexagongrid.fill"
        case .explore: return "rectangle.expand.vertical"
        case .improve: return "wand.and.stars"
        case .history: return "clock.fill"
        case .control: return "gearshape.fill"
        }
    }

    var destinations: [NavigationDestination] {
        switch self {
        case .home: return [.home]
        case .twin: return [.twinDashboard, .twinHardware, .twinSoftware, .twinFilesystem, .twinProcesses, .twinMemory, .twinEnergy, .twinApps, .twinReasoning, .twinManagement]
        case .explore: return [.exploreApplications, .exploreDisk, .exploreActivity]
        case .improve: return [.improveQuickScan, .improveClean, .improveNeuralScan, .improveZenMode, .improveAIAdvisor]
        case .history: return [.historyWhatChanged, .historyScanHistory]
        case .control: return [.controlAutomation, .controlSafety, .controlDiagnostics, .controlSettings]
        }
    }
}

enum NavigationDestination: String, CaseIterable, Identifiable, Hashable {
    // Home
    case home

    // Digital Twin — first-class section with all 10 twin dimensions
    case twinDashboard     // Overview: health scores, trust, anomalies
    case twinHardware      // CPU, GPU, Neural Engine, Memory, Storage, Battery
    case twinSoftware      // Software Genome: apps, frameworks, dylibs, kexts
    case twinFilesystem    // Filesystem Graph: size, duplicates, orphans, growth
    case twinProcesses     // Process Intelligence: tree, anomalies, bottlenecks
    case twinMemory        // Memory Intelligence: pressure, swap, leaks
    case twinEnergy        // Energy Twin: battery, consumers, thermal, wake
    case twinApps          // App Intelligence: health, crash probability, unused
    case twinReasoning     // Reasoning Engine: ask, predict, simulate, recommend
    case twinManagement    // Twin DB lifecycle, observers, compaction

    // Explore — non-twin system views
    case exploreApplications // App inventory (installed apps, sizes)
    case exploreDisk         // Disk usage (large files, directory breakdown)
    case exploreActivity     // Live activity: processes, RAM boost

    // Improve
    case improveQuickScan    // Quick composite scan
    case improveClean        // Clean scan + cleanup
    case improveNeuralScan   // GNN filesystem analysis
    case improveZenMode      // One-click optimization
    case improveAIAdvisor    // AI recommendations

    // History
    case historyWhatChanged  // Temporal diff
    case historyScanHistory  // Past scans

    // Control
    case controlAutomation   // Scheduled scans, launch agent
    case controlSafety       // Safety rules browser
    case controlDiagnostics  // CLI command tester
    case controlSettings     // App settings

    var id: String { rawValue }

    // MARK: - Metadata

    var label: String {
        switch self {
        case .home: return "Home"
        // Digital Twin
        case .twinDashboard: return "Dashboard"
        case .twinHardware: return "Hardware"
        case .twinSoftware: return "Software"
        case .twinFilesystem: return "Filesystem"
        case .twinProcesses: return "Processes"
        case .twinMemory: return "Memory"
        case .twinEnergy: return "Energy"
        case .twinApps: return "Apps"
        case .twinReasoning: return "Reasoning"
        case .twinManagement: return "Management"
        // Explore
        case .exploreApplications: return "Applications"
        case .exploreDisk: return "Disk Usage"
        case .exploreActivity: return "Activity"
        // Improve
        case .improveQuickScan: return "Quick Scan"
        case .improveClean: return "Clean"
        case .improveNeuralScan: return "Neural Scan"
        case .improveZenMode: return "Zen Mode"
        case .improveAIAdvisor: return "AI Advisor"
        // History
        case .historyWhatChanged: return "What Changed?"
        case .historyScanHistory: return "Scan History"
        // Control
        case .controlAutomation: return "Automation"
        case .controlSafety: return "Safety"
        case .controlDiagnostics: return "Diagnostics"
        case .controlSettings: return "Settings"
        }
    }

    var icon: String {
        switch self {
        case .home: return "house.fill"
        // Digital Twin
        case .twinDashboard: return "circle.hexagongrid.fill"
        case .twinHardware: return "cpu.fill"
        case .twinSoftware: return "shippingbox.fill"
        case .twinFilesystem: return "internaldrive"
        case .twinProcesses: return "gearshape.2.fill"
        case .twinMemory: return "memorychip.fill"
        case .twinEnergy: return "bolt.fill"
        case .twinApps: return "app.badge"
        case .twinReasoning: return "lightbulb.fill"
        case .twinManagement: return "externaldrive.connected.to.line.below"
        // Explore
        case .exploreApplications: return "app.badge"
        case .exploreDisk: return "internaldrive"
        case .exploreActivity: return "chart.bar.fill"
        // Improve
        case .improveQuickScan: return "bolt.circle"
        case .improveClean: return "trash.circle"
        case .improveNeuralScan: return "brain"
        case .improveZenMode: return "circle.hexagongrid"
        case .improveAIAdvisor: return "brain.head.profile"
        // History
        case .historyWhatChanged: return "arrow.triangle.2.circlepath"
        case .historyScanHistory: return "clock.arrow.circlepath"
        // Control
        case .controlAutomation: return "gearshape.arrow.triangle.2.circlepath"
        case .controlSafety: return "shield.lefthalf.filled"
        case .controlDiagnostics: return "stethoscope"
        case .controlSettings: return "gearshape"
        }
    }

    var section: NavigationSection {
        switch self {
        case .home: return .home
        case .twinDashboard, .twinHardware, .twinSoftware, .twinFilesystem,
             .twinProcesses, .twinMemory, .twinEnergy, .twinApps,
             .twinReasoning, .twinManagement: return .twin
        case .exploreApplications, .exploreDisk, .exploreActivity: return .explore
        case .improveQuickScan, .improveClean, .improveNeuralScan, .improveZenMode, .improveAIAdvisor: return .improve
        case .historyWhatChanged, .historyScanHistory: return .history
        case .controlAutomation, .controlSafety, .controlDiagnostics, .controlSettings: return .control
        }
    }

    // MARK: - Sub-destinations (only for Explore > Activity)

    var subDestinations: [SubDestination]? {
        switch self {
        case .exploreActivity:
            return [
                SubDestination(id: "processes", label: "Processes", icon: "gearshape.2.fill"),
                SubDestination(id: "ramboost", label: "RAM Boost", icon: "bolt.fill"),
            ]
        default:
            return nil
        }
    }
}

struct SubDestination: Hashable, Identifiable {
    let id: String
    let label: String
    let icon: String
}

// MARK: - AppRouter
//
// Single source of truth for navigation state.

@MainActor
final class AppRouter: ObservableObject {
    @Published var selectedDestination: NavigationDestination = .home
    @Published var selectedSubDestination: SubDestination?
    @Published var selectedSection: NavigationSection = .home

    func navigate(to destination: NavigationDestination) {
        selectedDestination = destination
        selectedSection = destination.section
        // Auto-select first sub-destination if available
        selectedSubDestination = destination.subDestinations?.first
    }

    func navigate(toSub sub: SubDestination) {
        selectedSubDestination = sub
    }

    // Convenience methods
    func goHome() { navigate(to: .home) }
    func goTwinDashboard() { navigate(to: .twinDashboard) }
    func goQuickScan() { navigate(to: .improveQuickScan) }
    func goClean() { navigate(to: .improveClean) }
    func goNeuralScan() { navigate(to: .improveNeuralScan) }
    func goZenMode() { navigate(to: .improveZenMode) }
    func goAIAdvisor() { navigate(to: .improveAIAdvisor) }
    func goWhatChanged() { navigate(to: .historyWhatChanged) }
    func goSettings() { navigate(to: .controlSettings) }
    func goDiagnostics() { navigate(to: .controlDiagnostics) }
}

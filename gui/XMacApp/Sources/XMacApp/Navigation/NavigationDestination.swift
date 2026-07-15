import SwiftUI

// MARK: - NavigationDestination
//
// Replaces the 35-case ScanMode enum with a clean, hierarchical navigation model.
// Five primary sections, each with clearly named destinations.
// Labels describe user outcomes, not internal engine names.

enum NavigationSection: String, CaseIterable, Identifiable {
    case home
    case explore
    case improve
    case history
    case control

    var id: String { rawValue }

    var label: String {
        switch self {
        case .home: return "Home"
        case .explore: return "Explore"
        case .improve: return "Improve"
        case .history: return "History"
        case .control: return "Control"
        }
    }

    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .explore: return "rectangle.expand.vertical"
        case .improve: return "wand.and.stars"
        case .history: return "clock.fill"
        case .control: return "gearshape.fill"
        }
    }

    var destinations: [NavigationDestination] {
        switch self {
        case .home: return [.home]
        case .explore: return [.exploreSystem, .exploreApplications, .exploreFilesystem, .exploreActivity]
        case .improve: return [.improveQuickScan, .improveClean, .improveNeuralScan, .improveZenMode, .improveAIAdvisor]
        case .history: return [.historyWhatChanged, .historyScanHistory]
        case .control: return [.controlAutomation, .controlSafety, .controlDiagnostics, .controlSettings]
        }
    }
}

enum NavigationDestination: String, CaseIterable, Identifiable, Hashable {
    // Home
    case home

    // Explore
    case exploreSystem       // Digital Twin: hardware, software, memory, energy
    case exploreApplications // App inventory
    case exploreFilesystem   // Disk, clean, depth, safety
    case exploreActivity     // Processes, memory, RAM boost, energy

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
        case .exploreSystem: return "System"
        case .exploreApplications: return "Applications"
        case .exploreFilesystem: return "Filesystem"
        case .exploreActivity: return "Activity"
        case .improveQuickScan: return "Quick Scan"
        case .improveClean: return "Clean"
        case .improveNeuralScan: return "Neural Scan"
        case .improveZenMode: return "Zen Mode"
        case .improveAIAdvisor: return "AI Advisor"
        case .historyWhatChanged: return "What Changed?"
        case .historyScanHistory: return "Scan History"
        case .controlAutomation: return "Automation"
        case .controlSafety: return "Safety"
        case .controlDiagnostics: return "Diagnostics"
        case .controlSettings: return "Settings"
        }
    }

    var icon: String {
        switch self {
        case .home: return "house.fill"
        case .exploreSystem: return "cpu.fill"
        case .exploreApplications: return "app.badge"
        case .exploreFilesystem: return "internaldrive"
        case .exploreActivity: return "chart.bar.fill"
        case .improveQuickScan: return "bolt.circle"
        case .improveClean: return "trash.circle"
        case .improveNeuralScan: return "brain"
        case .improveZenMode: return "circle.hexagongrid"
        case .improveAIAdvisor: return "brain.head.profile"
        case .historyWhatChanged: return "arrow.triangle.2.circlepath"
        case .historyScanHistory: return "clock.arrow.circlepath"
        case .controlAutomation: return "gearshape.arrow.triangle.2.circlepath"
        case .controlSafety: return "shield.lefthalf.filled"
        case .controlDiagnostics: return "stethoscope"
        case .controlSettings: return "gearshape"
        }
    }

    var section: NavigationSection {
        switch self {
        case .home: return .home
        case .exploreSystem, .exploreApplications, .exploreFilesystem, .exploreActivity: return .explore
        case .improveQuickScan, .improveClean, .improveNeuralScan, .improveZenMode, .improveAIAdvisor: return .improve
        case .historyWhatChanged, .historyScanHistory: return .history
        case .controlAutomation, .controlSafety, .controlDiagnostics, .controlSettings: return .control
        }
    }

    // MARK: - Sub-destinations for tabs that have them

    var subDestinations: [SubDestination]? {
        switch self {
        case .exploreSystem:
            return [
                SubDestination(id: "hardware", label: "Hardware", icon: "cpu"),
                SubDestination(id: "software", label: "Software", icon: "shippingbox.fill"),
                SubDestination(id: "memory", label: "Memory", icon: "memorychip.fill"),
                SubDestination(id: "energy", label: "Energy", icon: "bolt.fill"),
            ]
        case .exploreFilesystem:
            return [
                SubDestination(id: "disk", label: "Disk Usage", icon: "internaldrive"),
                SubDestination(id: "clean", label: "Clean", icon: "trash.circle"),
                SubDestination(id: "integrity", label: "Integrity", icon: "checkmark.shield"),
                SubDestination(id: "safety", label: "Safety", icon: "shield.lefthalf.filled"),
            ]
        case .exploreActivity:
            return [
                SubDestination(id: "processes", label: "Processes", icon: "gearshape.2.fill"),
                SubDestination(id: "memory", label: "Memory", icon: "memorychip.fill"),
                SubDestination(id: "ramboost", label: "RAM Boost", icon: "bolt.fill"),
                SubDestination(id: "energy", label: "Energy", icon: "flame.fill"),
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
// Replaces the scanMode property on XMacRunner.

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
    func goQuickScan() { navigate(to: .improveQuickScan) }
    func goClean() { navigate(to: .improveClean) }
    func goNeuralScan() { navigate(to: .improveNeuralScan) }
    func goZenMode() { navigate(to: .improveZenMode) }
    func goAIAdvisor() { navigate(to: .improveAIAdvisor) }
    func goWhatChanged() { navigate(to: .historyWhatChanged) }
    func goSettings() { navigate(to: .controlSettings) }
    func goDiagnostics() { navigate(to: .controlDiagnostics) }
}

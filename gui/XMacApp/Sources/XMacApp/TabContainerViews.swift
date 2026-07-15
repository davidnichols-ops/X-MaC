import SwiftUI

// MARK: - Filesystem Tab View
// Combines Disk Usage, Clean, Duplicates, FS Integrity into one tab

struct FilesystemTabView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var selectedSubTab: SubTab = .disk

    enum SubTab: String, CaseIterable, Identifiable {
        case disk = "Disk Usage"
        case clean = "Clean"
        case integrity = "FS Integrity"
        case depth = "Depth Scan"
        var id: String { rawValue }
        var icon: String {
            switch self {
            case .disk: return "internaldrive"
            case .clean: return "trash.circle"
            case .integrity: return "checkmark.shield"
            case .depth: return "scope"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Sub-tab picker
            HStack(spacing: 4) {
                ForEach(SubTab.allCases) { tab in
                    Button(action: { selectedSubTab = tab }) {
                        HStack(spacing: 6) {
                            Image(systemName: tab.icon)
                                .font(.system(size: 11))
                            Text(tab.rawValue)
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(selectedSubTab == tab ? XTheme.accent.opacity(0.2) : Color.clear)
                        .foregroundStyle(selectedSubTab == tab ? XTheme.accent : XTheme.textSecondary)
                        .cornerRadius(6)
                    }
                    .buttonStyle(.plain)
                }
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 10)

            Divider().background(XTheme.cardBorder)

            // Content
            switch selectedSubTab {
            case .disk:
                DiskView()
            case .clean:
                CleanView()
            case .integrity:
                DepthView()
            case .depth:
                DepthView()
            }
        }
        .background(XTheme.voidGradient)
    }
}

// MARK: - Activity Tab View
// Live monitoring: processes, threads, CPU, GPU, memory, IO, network

struct ActivityTabView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var selectedSubTab: SubTab = .processes

    enum SubTab: String, CaseIterable, Identifiable {
        case processes = "Processes"
        case memory = "Memory"
        case ramBoost = "RAM Boost"
        case energy = "Energy"
        var id: String { rawValue }
        var icon: String {
            switch self {
            case .processes: return "gearshape.2.fill"
            case .memory: return "memorychip.fill"
            case .ramBoost: return "bolt.fill"
            case .energy: return "flame.fill"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                ForEach(SubTab.allCases) { tab in
                    Button(action: { selectedSubTab = tab }) {
                        HStack(spacing: 6) {
                            Image(systemName: tab.icon)
                                .font(.system(size: 11))
                            Text(tab.rawValue)
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(selectedSubTab == tab ? XTheme.accent.opacity(0.2) : Color.clear)
                        .foregroundStyle(selectedSubTab == tab ? XTheme.accent : XTheme.textSecondary)
                        .cornerRadius(6)
                    }
                    .buttonStyle(.plain)
                }
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 10)

            Divider().background(XTheme.cardBorder)

            switch selectedSubTab {
            case .processes:
                TwinProcessView()
            case .memory:
                TwinMemoryView()
            case .ramBoost:
                RamBoostView()
            case .energy:
                TwinEnergyView()
            }
        }
        .background(XTheme.voidGradient)
    }
}

// MARK: - Optimization Tab View
// Smart recommendations, cleanup, performance, automation

struct OptimizationTabView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var selectedSubTab: SubTab = .zen

    enum SubTab: String, CaseIterable, Identifiable {
        case zen = "Zen Mode"
        case advisor = "AI Advisor"
        case reasoning = "Reasoning"
        case purge = "Purge"
        case quick = "Quick Scan"
        var id: String { rawValue }
        var icon: String {
            switch self {
            case .zen: return "circle.hexagongrid"
            case .advisor: return "brain.head.profile"
            case .reasoning: return "lightbulb.fill"
            case .purge: return "trash.fill"
            case .quick: return "bolt.circle"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 4) {
                ForEach(SubTab.allCases) { tab in
                    Button(action: { selectedSubTab = tab }) {
                        HStack(spacing: 6) {
                            Image(systemName: tab.icon)
                                .font(.system(size: 11))
                            Text(tab.rawValue)
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(selectedSubTab == tab ? XTheme.accent.opacity(0.2) : Color.clear)
                        .foregroundStyle(selectedSubTab == tab ? XTheme.accent : XTheme.textSecondary)
                        .cornerRadius(6)
                    }
                    .buttonStyle(.plain)
                }
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 10)

            Divider().background(XTheme.cardBorder)

            switch selectedSubTab {
            case .zen:
                ZenView()
            case .advisor:
                AdvisorView()
            case .reasoning:
                TwinReasoningView()
            case .purge:
                PurgeView()
            case .quick:
                QuickScanView()
            }
        }
        .background(XTheme.voidGradient)
    }
}

// MARK: - Timeline Tab View
// Combines Event Timeline and What Changed? into one tab

struct TimelineTabView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var selectedSubTab: SubTab = .events

    enum SubTab: String, CaseIterable, Identifiable {
        case events = "Event Timeline"
        case whatChanged = "What Changed?"
        var id: String { rawValue }
        var icon: String {
            switch self {
            case .events: return "timeline"
            case .whatChanged: return "arrow.triangle.2.circlepath"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Sub-tab picker
            HStack(spacing: 4) {
                ForEach(SubTab.allCases) { tab in
                    Button(action: { selectedSubTab = tab }) {
                        HStack(spacing: 6) {
                            Image(systemName: tab.icon)
                                .font(.system(size: 11))
                            Text(tab.rawValue)
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(selectedSubTab == tab ? XTheme.accent.opacity(0.2) : Color.clear)
                        .foregroundStyle(selectedSubTab == tab ? XTheme.accent : XTheme.textSecondary)
                        .cornerRadius(6)
                    }
                    .buttonStyle(.plain)
                }
                Spacer()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 10)

            Divider().background(XTheme.cardBorder)

            // Content
            switch selectedSubTab {
            case .events:
                TimelineView()
            case .whatChanged:
                WhatChangedView()
            }
        }
        .background(XTheme.voidGradient)
    }
}

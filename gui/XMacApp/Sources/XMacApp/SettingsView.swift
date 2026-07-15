import SwiftUI
import CoreML

struct SettingsView: View {
    @EnvironmentObject var runner: XMacRunner
    @EnvironmentObject var settings: AppSettings
    @EnvironmentObject var profiles: ProfileStore
    @State private var profileName: String = ""
    @State private var showingImportSheet = false
    @State private var importJSON = ""
    @State private var showingDeleteConfirmation = false

    private var xmacPath: String {
        let candidates = [
            Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/xmac").path,
            "/opt/homebrew/bin/xmac",
            "/usr/local/bin/xmac",
        ]
        for p in candidates {
            if FileManager.default.isExecutableFile(atPath: p) {
                return p
            }
        }
        return "xmac (not found in PATH)"
    }

    private var modelStatus: (loaded: Bool, path: String) {
        if Bundle.main.url(forResource: "XMacGNN", withExtension: "mlpackage") != nil {
            return (true, "XMacGNN.mlpackage found in bundle")
        }

        let cacheDir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("XMacGNN", isDirectory: true)
            .appendingPathComponent("XMacGNN.mlmodelc")

        if FileManager.default.fileExists(atPath: cacheDir.path) {
            return (true, "Compiled model found in cache")
        }

        return (false, "XMacGNN.mlpackage not found")
    }

    private var appVersion: String {
        let v = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0"
        let b = Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "1"
        return "v\(v) (\(b))"
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                SettingsSectionCard(title: "Binary", icon: "terminal") {
                    SettingsRow(label: "xmac Path", value: xmacPath)
                }

                SettingsSectionCard(title: "Cleanup Profiles", icon: "person.2") {
                    Picker("Profile", selection: Binding(
                        get: { profiles.selectedProfileId },
                        set: { profiles.selectedProfileId = $0 }
                    )) {
                        ForEach(profiles.profiles) { profile in
                            Text(profile.name).tag(profile.id)
                        }
                    }
                    .pickerStyle(.menu)
                    .disabled(runner.isScanning)

                    Text(profiles.selectedProfile.description)
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)

                    HStack {
                        Button("Duplicate") {
                            let copy = profiles.duplicate(profiles.selectedProfile)
                            profiles.add(copy)
                        }
                        .disabled(profiles.selectedProfile.isBuiltIn == false)
                        Button("Export") {
                            if let json = profiles.exportSelected() {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(json, forType: .string)
                            }
                        }
                        Button("Import") { showingImportSheet = true }
                        Button("Delete") {
                            showingDeleteConfirmation = true
                        }
                        .disabled(profiles.selectedProfile.isBuiltIn)
                    }
                    .controlSize(.small)
                    .confirmationDialog(
                        "Delete profile \"\(profiles.selectedProfile.name)\"?",
                        isPresented: $showingDeleteConfirmation,
                        titleVisibility: .visible
                    ) {
                        Button("Delete Profile", role: .destructive) {
                            profiles.delete(profiles.selectedProfile)
                        }
                        Button("Cancel", role: .cancel) {}
                    } message: {
                        Text("This action cannot be undone. The profile and all its settings will be permanently removed.")
                    }
                }

                SettingsSectionCard(title: "Per-category policy", icon: "checklist") {
                    PerCategoryPolicyEditor(profile: $profiles.selectedProfile)
                }

                SettingsSectionCard(title: "Cleanup policy", icon: "shield.checkered") {
                    Toggle("Confirm before cleanup", isOn: $settings.confirmCleanup)
                    Toggle("Move to Trash instead of permanent delete", isOn: $settings.trashInsteadOfDelete)
                    Stepper("Minimum age: \(settings.minimumAgeDays) days", value: $settings.minimumAgeDays, in: 1...365)
                    Stepper("Minimum size: \(settings.minimumSizeMB) MB", value: $settings.minimumSizeMB, in: 0...10000)
                    Toggle("Include hidden files", isOn: $settings.includeHidden)
                    Toggle("Follow symbolic links", isOn: $settings.followSymlinks)
                }

                SettingsSectionCard(title: "Appearance", icon: "paintbrush") {
                    Picker("Theme", selection: $settings.appearance) {
                        Text("System").tag("system")
                        Text("Light").tag("light")
                        Text("Dark").tag("dark")
                    }
                    .pickerStyle(.segmented)
                }

                SettingsSectionCard(title: "Exclusions", icon: "slash.circle") {
                    Text("One path per line. Exclusions are passed to scans and cannot be cleaned.")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                    TextEditor(text: $settings.excludePaths)
                        .font(.system(size: 11, design: .monospaced))
                        .frame(minHeight: 90)
                        .padding(6)
                        .background(XTheme.bgTertiary)
                        .clipShape(RoundedRectangle(cornerRadius: 7))
                }

                SettingsSectionCard(title: "CoreML Model", icon: "cpu") {
                    HStack(spacing: 10) {
                        Image(systemName: modelStatus.loaded ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .font(.system(size: 14))
                            .foregroundStyle(modelStatus.loaded ? XTheme.safe : XTheme.danger)

                        VStack(alignment: .leading, spacing: 2) {
                            Text(modelStatus.loaded ? "Loaded" : "Not Loaded")
                                .font(.system(size: 13, weight: .semibold))
                                .foregroundStyle(modelStatus.loaded ? XTheme.safe : XTheme.danger)
                            Text(modelStatus.path)
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textTertiary)
                        }

                        Spacer()
                    }
                    .padding(.vertical, 4)
                }

                SettingsSectionCard(title: "About", icon: "info.circle") {
                    SettingsRow(label: "App", value: "X-MaC")
                    SettingsRow(label: "Version", value: appVersion)
                    SettingsRow(label: "Description", value: "macOS cleaner, optimizer, and system scanner")
                    SettingsRow(label: "Engine", value: "Rust + CoreML GNN")
                    SettingsRow(label: "GNN Accuracy", value: "99.74%")
                }

                SettingsSectionCard(title: "What's New in 2.0", icon: "sparkles") {
                    VStack(alignment: .leading, spacing: 8) {
                        whatsNewRow(icon: "bolt.fill", text: "RAM Boost — purge inactive memory, kill hungry processes, before/after comparison")
                        whatsNewRow(icon: "brain.head.profile", text: "GNN Neural Scan — 99.74% accuracy, 27 file categories, on-device CoreML")
                        whatsNewRow(icon: "externaldrive.badge.timemachine", text: "Time Machine & backup volume protection across all engines")
                        whatsNewRow(icon: "shield.lefthalf.filled", text: "Backup-aware cleanup policies and fix script generation")
                        whatsNewRow(icon: "wrench.and.screwdriver", text: "Maintenance tasks: DNS flush, Spotlight, LaunchServices, RAM purge")
                    }
                }

                Spacer()
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
        .sheet(isPresented: $showingImportSheet) {
            ImportProfileSheet(importJSON: $importJSON, profiles: profiles, isPresented: $showingImportSheet)
        }
    }

    private func whatsNewRow(icon: String, text: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 11))
                .foregroundStyle(XTheme.accent)
                .frame(width: 16)
            Text(text)
                .font(.system(size: 11))
                .foregroundStyle(XTheme.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

// MARK: - Section Card

struct SettingsSectionCard<Content: View>: View {
    let title: String
    let icon: String
    @ViewBuilder let content: Content

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: title, icon: icon)

                content
            }
        }
    }
}

// MARK: - Settings Row

struct SettingsRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Text(label)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(XTheme.textTertiary)
                .frame(width: 100, alignment: .leading)

            Text(value)
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textPrimary)
                .textSelection(.enabled)

            Spacer()
        }
        .padding(.vertical, 3)
    }
}

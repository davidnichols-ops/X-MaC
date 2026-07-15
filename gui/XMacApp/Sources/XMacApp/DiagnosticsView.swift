import SwiftUI

// MARK: - Diagnostics View
// Observability/action panel that lets you directly run every CLI subcommand
// and see the raw JSON output. Maps each command to the 630 operations it covers.

struct DiagnosticsView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var results: [String: DiagResult] = [:]
    @State private var runningCommand: String? = nil
    @State private var rawOutput: String = ""
    @State private var selectedCommand: String? = nil
    @State private var filterText = ""

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Header
                diagHeader

                // Run All button
                runAllSection

                // Command categories
                ForEach(DiagCategory.allCases) { category in
                    categorySection(category)
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }

    // MARK: - Header

    private var diagHeader: some View {
        HStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Diagnostics & Operations Panel")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("Directly run every CLI subcommand and verify JSON output")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
            VStack(spacing: 4) {
                Text("\(results.values.filter { $0.status == .pass }.count)/\(DiagCommand.all.count) passed")
                    .font(.system(size: 16, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.safe)
                Text("\(results.values.filter { $0.status == .fail }.count) failed, \(results.values.filter { $0.status == .pending }.count) pending")
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
            }
        }
    }

    // MARK: - Run All

    private var runAllSection: some View {
        XCard {
            HStack(spacing: 12) {
                Button("Run All Tests") { runAllTests() }
                    .buttonStyle(.borderedProminent)
                    .tint(XTheme.accent)
                    .disabled(runningCommand != nil)

                Button("Clear Results") { results.removeAll(); rawOutput = "" }
                    .buttonStyle(.bordered)
                    .tint(XTheme.medium)

                Spacer()

                if let cmd = runningCommand {
                    HStack(spacing: 6) {
                        ProgressView()
                            .scaleEffect(0.7)
                        Text("Running: \(cmd)...")
                            .font(.system(size: 12))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                }
            }
        }
    }

    // MARK: - Category Section

    private func categorySection(_ category: DiagCategory) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 8) {
                    Image(systemName: category.icon)
                        .font(.system(size: 16))
                        .foregroundStyle(XTheme.accent)
                    Text(category.title)
                        .font(.system(size: 15, weight: .bold))
                        .foregroundStyle(XTheme.textPrimary)
                    Text(category.opRange)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(XTheme.textTertiary)
                    Spacer()
                    let catResults = category.commands.compactMap { results[$0.id] }
                    let passCount = catResults.filter { $0.status == .pass }.count
                    let totalCount = category.commands.count
                    Text("\(passCount)/\(totalCount)")
                        .font(.system(size: 12, weight: .bold, design: .monospaced))
                        .foregroundStyle(passCount == totalCount ? XTheme.safe : passCount > 0 ? XTheme.medium : XTheme.textTertiary)
                }

                ForEach(category.commands) { cmd in
                    commandRow(cmd)
                }
            }
        }
    }

    // MARK: - Command Row

    private func commandRow(_ cmd: DiagCommand) -> some View {
        let result = results[cmd.id]
        let isSelected = selectedCommand == cmd.id

        return VStack(spacing: 0) {
            HStack(spacing: 10) {
                // Status indicator
                statusIcon(result?.status ?? .pending)

                // Command info
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(cmd.name)
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                        Text(cmd.cliCommand)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    Text(cmd.description)
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                        .lineLimit(1)
                }

                Spacer()

                // Run button
                Button("Run") { runCommand(cmd) }
                    .buttonStyle(.bordered)
                    .tint(XTheme.accent)
                    .controlSize(.small)
                    .disabled(runningCommand != nil)

                // Result info
                if let r = result {
                    Text(r.summary)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(r.status == .pass ? XTheme.safe : r.status == .fail ? XTheme.danger : XTheme.textTertiary)
                }
            }
            .padding(.vertical, 6)
            .contentShape(Rectangle())
            .onTapGesture { selectedCommand = isSelected ? nil : cmd.id }

            // Show raw output for selected command
            if isSelected && result != nil {
                rawOutputView(result!)
            }
        }
        .background(
            isSelected ? XTheme.accent.opacity(0.05) : Color.clear
        )
        .cornerRadius(6)
    }

    // MARK: - Status Icon

    private func statusIcon(_ status: DiagStatus) -> some View {
        let (icon, color) = statusIconData(status)
        return Image(systemName: icon)
            .font(.system(size: 12))
            .foregroundStyle(color)
            .frame(width: 16)
    }

    private func statusIconData(_ status: DiagStatus) -> (String, Color) {
        switch status {
        case .pass: return ("checkmark.circle.fill", XTheme.safe)
        case .fail: return ("xmark.circle.fill", XTheme.danger)
        case .running: return ("arrow.triangle.2.circlepath", XTheme.accent)
        case .pending: return ("circle", XTheme.textTertiary)
        }
    }

    // MARK: - Raw Output View

    private func rawOutputView(_ result: DiagResult) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Text("Exit code: \(result.exitCode)")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(XTheme.textTertiary)
                Text("Duration: \(String(format: "%.2f", result.durationSecs))s")
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(XTheme.textTertiary)
                if result.jsonValid {
                    Text("JSON: valid")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(XTheme.safe)
                } else {
                    Text("JSON: invalid")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(XTheme.danger)
                }
                if result.findingCount > 0 {
                    Text("Findings: \(result.findingCount)")
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(XTheme.accent)
                }
            }

            // Output preview (first 500 chars)
            ScrollView {
                Text(result.outputPreview)
                    .font(.system(size: 10, design: .monospaced))
                    .foregroundStyle(XTheme.textSecondary)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .textSelection(.enabled)
            }
            .frame(maxHeight: 200)
            .padding(8)
            .background(XTheme.color.backgroundCanvas.opacity(0.3))
            .cornerRadius(6)
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 4)
    }

    // MARK: - Actions

    private func runCommand(_ cmd: DiagCommand) {
        runningCommand = cmd.id
        results[cmd.id] = DiagResult(status: .running, exitCode: -1, durationSecs: 0, outputPreview: "", jsonValid: false, findingCount: 0, summary: "running...")

        Task {
            let start = Date()
            let args = [runner.xmacPath] + cmd.cliArgs
            let (exitCode, output) = await runCliCommand(args)
            let duration = Date().timeIntervalSince(start)

            let jsonValid = validateJson(output)
            let findingCount = countFindings(output)
            let status: DiagStatus = exitCode == 0 && (jsonValid || cmd.acceptsNonJson) ? .pass : .fail
            let preview = String(output.prefix(2000))
            let summary = status == .pass ? "✓ \(durationSecs(start))s" : "✗ exit=\(exitCode)"

            await MainActor.run {
                results[cmd.id] = DiagResult(
                    status: status,
                    exitCode: exitCode,
                    durationSecs: duration,
                    outputPreview: preview,
                    jsonValid: jsonValid,
                    findingCount: findingCount,
                    summary: summary
                )
                runningCommand = nil
            }
        }
    }

    private func runAllTests() {
        let allCommands = DiagCommand.all
        Task {
            for cmd in allCommands {
                await MainActor.run { runCommand(cmd) }
                // Wait for completion
                while runningCommand != nil {
                    try? await Task.sleep(nanoseconds: 100_000_000)
                }
            }
        }
    }

    // MARK: - Helpers

    private func runCliCommand(_ args: [String]) async -> (Int, String) {
        let task = Process()
        task.executableURL = URL(fileURLWithPath: args[0])
        task.arguments = Array(args.dropFirst())
        let pipe = Pipe()
        task.standardOutput = pipe
        task.standardError = Pipe() // discard stderr

        do {
            try task.run()
        } catch {
            return (-1, "Failed to launch: \(error.localizedDescription)")
        }

        // Wait with timeout
        let timeoutSecs = 60.0
        let deadline = Date().addingTimeInterval(timeoutSecs)
        while task.isRunning == true && Date() < deadline {
            try? await Task.sleep(nanoseconds: 100_000_000)
        }
        if task.isRunning == true {
            task.terminate()
            return (-1, "TIMEOUT after \(Int(timeoutSecs))s")
        }

        let data = pipe.fileHandleForReading.readDataToEndOfFile()
        let output = String(data: data, encoding: .utf8) ?? "<binary output>"
        return (Int(task.terminationStatus), output)
    }

    private func validateJson(_ output: String) -> Bool {
        guard !output.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return false }
        // Try to parse as JSON (could be NDJSON or single JSON object)
        let trimmed = output.trimmingCharacters(in: .whitespacesAndNewlines)
        // Try single JSON
        if let _ = try? JSONSerialization.jsonObject(with: trimmed.data(using: .utf8) ?? Data()) {
            return true
        }
        // Try NDJSON (each line is a JSON object)
        let lines = trimmed.split(separator: "\n").filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }
        if lines.count > 1 {
            let validLines = lines.filter { line in
                (try? JSONSerialization.jsonObject(with: String(line).data(using: .utf8) ?? Data())) != nil
            }
            return validLines.count > 0 && validLines.count >= lines.count / 2
        }
        return false
    }

    private func countFindings(_ output: String) -> Int {
        let lines = output.split(separator: "\n").filter { line in
            let s = line.trimmingCharacters(in: .whitespaces)
            return s.hasPrefix("{") && s.contains("\"engine\"")
        }
        return lines.count
    }

    private func durationSecs(_ start: Date) -> String {
        let secs = Date().timeIntervalSince(start)
        if secs < 60 { return String(format: "%.1f", secs) }
        return String(format: "%.0f", secs)
    }
}

// MARK: - Data Models

struct DiagResult {
    let status: DiagStatus
    let exitCode: Int
    let durationSecs: Double
    let outputPreview: String
    let jsonValid: Bool
    let findingCount: Int
    let summary: String
}

enum DiagStatus: String {
    case pending
    case running
    case pass
    case fail
}

// MARK: - Command Definitions

enum DiagCategory: String, CaseIterable, Identifiable {
    case filesystem = "Filesystem & Storage"
    case cache = "Cache & Cleanup"
    case env = "Environment Mapping"
    case system = "System Maintenance"
    case memory = "Memory & Optimization"
    case twin = "Digital Twin"
    case cleanup = "Cleanup & History"
    case config = "Configuration"
    case composite = "Composite Commands"

    var id: String { rawValue }

    var title: String { rawValue }

    var icon: String {
        switch self {
        case .filesystem: return "internaldrive.fill"
        case .cache: return "trash.fill"
        case .env: return "map.fill"
        case .system: return "wrench.and.screwdriver.fill"
        case .memory: return "memorychip.fill"
        case .twin: return "brain.fill"
        case .cleanup: return "checkmark.shield.fill"
        case .config: return "gearshape.fill"
        case .composite: return "wand.and.stars"
        }
    }

    var opRange: String {
        switch self {
        case .filesystem: return "Ops 1-30"
        case .cache: return "Ops 31-140, 261-300"
        case .env: return "Ops 141-170"
        case .system: return "Ops 171-200"
        case .memory: return "Ops 171-200"
        case .twin: return "Ops 201-600"
        case .cleanup: return "Ops 601-630"
        case .config: return "Config"
        case .composite: return "Composite"
        }
    }

    var commands: [DiagCommand] {
        DiagCommand.all.filter { $0.category == self }
    }
}

struct DiagCommand: Identifiable, Hashable {
    let id: String
    let name: String
    let cliCommand: String
    let cliArgs: [String]
    let description: String
    let category: DiagCategory
    let acceptsNonJson: Bool

    init(id: String, name: String, cliArgs: [String], description: String, category: DiagCategory, acceptsNonJson: Bool = false) {
        self.id = id
        self.name = name
        self.cliCommand = cliArgs.filter { $0 != "--format" && $0 != "json" }.joined(separator: " ")
        self.cliArgs = cliArgs
        self.description = description
        self.category = category
        self.acceptsNonJson = acceptsNonJson
    }

    static let all: [DiagCommand] = [
        // Filesystem & Storage (Ops 1-30)
        DiagCommand(
            id: "disk",
            name: "Disk Usage",
            cliArgs: ["--format", "json", "disk", "--top", "30", "--min-size", "50M"],
            description: "Top directories and files by size",
            category: .filesystem
        ),
        DiagCommand(
            id: "depth",
            name: "Filesystem Integrity",
            cliArgs: ["--format", "json", "depth"],
            description: "Permissions, symlinks, dylib dependencies",
            category: .filesystem
        ),
        DiagCommand(
            id: "graph",
            name: "GNN Graph Extraction",
            cliArgs: ["--format", "json", "graph", "--max-depth", "5", "--max-nodes", "500"],
            description: "File system graph for GNN training",
            category: .filesystem
        ),

        // Cache & Cleanup (Ops 31-140, 261-300)
        DiagCommand(
            id: "clean",
            name: "Clean Scan",
            cliArgs: ["--format", "json", "clean", "--min-age", "30d", "--min-size", "1M"],
            description: "Find reclaimable disk space (caches, builds, trash)",
            category: .cache
        ),
        DiagCommand(
            id: "clean-xcode",
            name: "Xcode Artifacts",
            cliArgs: ["--format", "json", "clean", "--xcode", "true", "--cache-dir", "false", "--browser", "false", "--trash", "false", "--temp", "false", "--build-artifacts", "false", "--pkg-caches", "false", "--docker", "false", "--ios-backups", "false", "--languages", "false", "--large-files", "false", "--mail", "false"],
            description: "Xcode-derived data and build artifacts only",
            category: .cache
        ),
        DiagCommand(
            id: "clean-browser",
            name: "Browser Cache",
            cliArgs: ["--format", "json", "clean", "--browser", "true", "--cache-dir", "false", "--xcode", "false", "--trash", "false", "--temp", "false", "--build-artifacts", "false", "--pkg-caches", "false", "--docker", "false", "--ios-backups", "false", "--languages", "false", "--large-files", "false", "--mail", "false"],
            description: "Safari, Chrome, Firefox cache only",
            category: .cache
        ),
        DiagCommand(
            id: "clean-trash",
            name: "Trash Bin",
            cliArgs: ["--format", "json", "clean", "--trash", "true", "--cache-dir", "false", "--browser", "false", "--xcode", "false", "--temp", "false", "--build-artifacts", "false", "--pkg-caches", "false", "--docker", "false", "--ios-backups", "false", "--languages", "false", "--large-files", "false", "--mail", "false"],
            description: "Files in Trash only",
            category: .cache
        ),
        DiagCommand(
            id: "clean-ios",
            name: "iOS Backups",
            cliArgs: ["--format", "json", "clean", "--ios-backups", "true", "--cache-dir", "false", "--browser", "false", "--trash", "false", "--temp", "false", "--build-artifacts", "false", "--pkg-caches", "false", "--docker", "false", "--xcode", "false", "--languages", "false", "--large-files", "false", "--mail", "false"],
            description: "Old iOS device backups only",
            category: .cache
        ),
        DiagCommand(
            id: "clean-large",
            name: "Large Files",
            cliArgs: ["--format", "json", "clean", "--large-files", "true", "--min-large-size", "500M", "--cache-dir", "false", "--browser", "false", "--trash", "false", "--temp", "false", "--build-artifacts", "false", "--pkg-caches", "false", "--docker", "false", "--ios-backups", "false", "--xcode", "false", "--languages", "false", "--mail", "false"],
            description: "Files larger than 500MB",
            category: .cache
        ),
        DiagCommand(
            id: "clean-dedup",
            name: "Duplicate Detection",
            cliArgs: ["--format", "json", "clean", "--dedup", "--cache-dir", "false", "--browser", "false", "--trash", "false", "--temp", "false", "--build-artifacts", "false", "--pkg-caches", "false", "--docker", "false", "--ios-backups", "false", "--xcode", "false", "--languages", "false", "--large-files", "false", "--mail", "false"],
            description: "Find duplicate files via BLAKE3 hashing",
            category: .cache
        ),

        // Environment Mapping (Ops 141-170)
        DiagCommand(
            id: "conflict",
            name: "PATH Conflicts",
            cliArgs: ["--format", "json", "conflict"],
            description: "PATH conflicts, env var conflicts, port usage",
            category: .env
        ),
        DiagCommand(
            id: "map",
            name: "Environment Map",
            cliArgs: ["--format", "json", "map"],
            description: "Python/Node.js environments and container runtimes",
            category: .env
        ),
        DiagCommand(
            id: "envmap",
            name: "System Environment",
            cliArgs: ["--format", "json", "envmap"],
            description: "OS, packages, installed apps (privacy-first)",
            category: .env
        ),

        // System Maintenance (Ops 171-200)
        DiagCommand(
            id: "maintain",
            name: "Maintenance Tasks",
            cliArgs: ["--format", "json", "maintain"],
            description: "Flush DNS, reindex Spotlight, periodic scripts",
            category: .system
        ),

        // Memory & Optimization (Ops 171-200)
        DiagCommand(
            id: "ram-boost",
            name: "RAM Boost (Report)",
            cliArgs: ["--format", "json", "ram-boost", "--report-only"],
            description: "Memory usage report (read-only)",
            category: .memory
        ),
        DiagCommand(
            id: "optimize",
            name: "Memory Optimization",
            cliArgs: ["--format", "json", "optimize", "--observe", "true"],
            description: "Memory telemetry, graph, pressure prediction",
            category: .memory
        ),

        // Digital Twin (Ops 201-600)
        DiagCommand(
            id: "twin-collect",
            name: "Twin: Collect",
            cliArgs: ["--format", "json", "twin", "--action", "collect"],
            description: "Full Digital Twin snapshot (all 8 dimensions)",
            category: .twin
        ),
        DiagCommand(
            id: "twin-simulate",
            name: "Twin: Simulate",
            cliArgs: ["--format", "json", "twin", "--action", "simulate", "--simulate", "clear cache"],
            description: "Sandbox simulation of an action",
            category: .twin
        ),
        DiagCommand(
            id: "twin-ask",
            name: "Twin: Ask",
            cliArgs: ["--format", "json", "twin", "--action", "ask", "--question", "How is my system?"],
            description: "Ask the reasoning engine a question",
            category: .twin
        ),
        DiagCommand(
            id: "twin-predict",
            name: "Twin: Predict",
            cliArgs: ["--format", "json", "twin", "--action", "predict"],
            description: "Predict future problems",
            category: .twin
        ),
        DiagCommand(
            id: "twin-recommend",
            name: "Twin: Recommend",
            cliArgs: ["--format", "json", "twin", "--action", "recommend"],
            description: "Get optimization recommendations",
            category: .twin
        ),
        DiagCommand(
            id: "twin-query-health",
            name: "Twin: Query Health",
            cliArgs: ["--format", "json", "twin", "--action", "query", "--query", "health"],
            description: "Query health dimension",
            category: .twin
        ),
        DiagCommand(
            id: "twin-benchmark",
            name: "Twin: Benchmark",
            cliArgs: ["--format", "json", "twin", "--action", "benchmark"],
            description: "Generate anonymized benchmark data",
            category: .twin
        ),
        DiagCommand(
            id: "twin-monitor",
            name: "Twin: Monitor",
            cliArgs: ["--format", "json", "twin", "--action", "monitor"],
            description: "Continuous monitoring plan",
            category: .twin
        ),

        // Cleanup & History (Ops 601-630)
        DiagCommand(
            id: "history",
            name: "Scan History",
            cliArgs: ["--format", "json", "history"],
            description: "View cleanup and scan history",
            category: .cleanup
        ),
        DiagCommand(
            id: "history-summary",
            name: "History Summary",
            cliArgs: ["--format", "json", "history", "--summary"],
            description: "Summary of cleanup history",
            category: .cleanup
        ),

        // Configuration
        DiagCommand(
            id: "config-show",
            name: "Config: Show",
            cliArgs: ["--format", "json", "config", "show"],
            description: "Show current configuration",
            category: .config
        ),
        DiagCommand(
            id: "config-path",
            name: "Config: Path",
            cliArgs: ["--format", "json", "config", "path"],
            description: "Show config file path",
            category: .config
        ),
        DiagCommand(
            id: "config-profiles",
            name: "Config: Profiles",
            cliArgs: ["--format", "json", "config", "profiles"],
            description: "List available optimization profiles",
            category: .config
        ),

        // Composite Commands
        DiagCommand(
            id: "quick",
            name: "Quick Scan",
            cliArgs: ["--format", "json", "quick"],
            description: "One-shot: clean + maintain + disk breakdown",
            category: .composite
        ),
        DiagCommand(
            id: "scan",
            name: "Full Scan",
            cliArgs: ["--format", "json", "scan"],
            description: "Full system scan (clean + conflict + map + envmap)",
            category: .composite
        ),
        DiagCommand(
            id: "all",
            name: "All Engines",
            cliArgs: ["--format", "json", "all"],
            description: "Run all engines including depth",
            category: .composite
        ),
        DiagCommand(
            id: "zen",
            name: "Zen Mode (Preview)",
            cliArgs: ["--format", "json", "zen"],
            description: "Comprehensive optimization preview",
            category: .composite
        ),
        DiagCommand(
            id: "advisor",
            name: "AI Advisor",
            cliArgs: ["--format", "json", "advisor", "--advisor-format", "json", "--top", "10"],
            description: "Natural-language system recommendations",
            category: .composite
        ),
    ]
}

import SwiftUI

struct MaintainView: View {
    @EnvironmentObject var runner: XMacRunner

    private var maintainFindings: [Finding] {
        runner.findings.filter { $0.category == "system_maintenance" }
    }

    private var successCount: Int {
        maintainFindings.filter { $0.severity == "info" }.count
    }

    private var failedCount: Int {
        maintainFindings.filter { $0.severity == "medium" || $0.severity == "high" }.count
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if runner.isScanning {
                    ScanProgressView(phase: runner.scanPhase)
                } else if let err = runner.error {
                    ScanErrorView(message: err)
                } else if maintainFindings.isEmpty {
                    EmptyScanView(message: "No maintenance tasks found. Run a maintain scan to see results.")
                } else {
                    MaintainSummaryCard(success: successCount, failed: failedCount, total: maintainFindings.count)

                    MaintainTaskList(findings: maintainFindings)
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }
}

// MARK: - Summary Card

struct MaintainSummaryCard: View {
    let success: Int
    let failed: Int
    let total: Int

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Maintenance Summary", icon: "wrench.and.screwdriver", count: total)

                HStack(spacing: 24) {
                    StatBadge(
                        icon: "checkmark.circle.fill",
                        label: "Succeeded",
                        value: "\(success)",
                        color: XTheme.safe
                    )
                    StatBadge(
                        icon: "exclamationmark.triangle.fill",
                        label: "Failed",
                        value: "\(failed)",
                        color: XTheme.medium
                    )
                    StatBadge(
                        icon: "list.bullet",
                        label: "Total Tasks",
                        value: "\(total)",
                        color: XTheme.accent
                    )
                }
            }
        }
    }
}

// MARK: - Task List

struct MaintainTaskList: View {
    let findings: [Finding]

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Maintenance Tasks", icon: "list.bullet.clipboard", count: findings.count)

                LazyVStack(spacing: 10) {
                    ForEach(findings) { finding in
                        MaintainTaskRow(finding: finding)
                    }
                }
            }
        }
    }
}

// MARK: - Task Row

struct MaintainTaskRow: View {
    let finding: Finding

    @State private var fixingTask: String? = nil
    @State private var fixResult: String? = nil

    private var isSuccess: Bool {
        finding.severity == "info"
    }

    private var isFixable: Bool {
        !isSuccess
    }

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: isSuccess ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                .font(.system(size: 16, weight: .medium))
                .foregroundStyle(isSuccess ? XTheme.safe : XTheme.medium)
                .frame(width: 22, height: 22)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(finding.title)
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                        .lineLimit(1)

                    Spacer()

                    if isFixable {
                        fixButton
                    }

                    Text(isSuccess ? "Success" : "Failed")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(isSuccess ? XTheme.safe : XTheme.medium)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background((isSuccess ? XTheme.safe : XTheme.medium).opacity(0.15))
                        .clipShape(Capsule())
                }

                Text(finding.description)
                    .font(.system(size: 11))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(2)

                if let hint = finding.remediation_hint, !hint.isEmpty {
                    HStack(spacing: 4) {
                        Image(systemName: "lightbulb")
                            .font(.system(size: 9))
                            .foregroundStyle(XTheme.warning)
                        Text(hint)
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    .padding(.top, 2)
                }

                if let result = fixResult {
                    HStack(spacing: 4) {
                        Image(systemName: fixingTask == nil ? "checkmark.circle" : "info.circle")
                            .font(.system(size: 9))
                            .foregroundStyle(fixingTask == nil ? XTheme.safe : XTheme.accent)
                        Text(result)
                            .font(.system(size: 10))
                            .foregroundStyle(fixingTask == nil ? XTheme.safe : XTheme.accent)
                    }
                    .padding(.top, 2)
                }
            }
        }
        .padding(.vertical, 6)
        .overlay(
            Rectangle()
                .frame(height: 1)
                .foregroundStyle(XTheme.cardBorder)
                .padding(.leading, 34),
            alignment: .bottom
        )
    }

    private var fixButton: some View {
        Button {
            runFix()
        } label: {
            HStack(spacing: 4) {
                if fixingTask == finding.id {
                    ProgressView()
                        .scaleEffect(0.6)
                        .frame(width: 12, height: 12)
                } else {
                    Image(systemName: "wrench.and.screwdriver")
                        .font(.system(size: 10))
                }
                Text("Fix")
                    .font(.system(size: 10, weight: .semibold))
            }
            .foregroundStyle(XTheme.accent)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(XTheme.accent.opacity(0.12))
            .clipShape(Capsule())
        }
        .buttonStyle(.plain)
        .disabled(fixingTask == finding.id)
    }

    private func runFix() {
        fixResult = nil
        fixingTask = finding.id

        Task { @MainActor in
            let result = await performFix(for: finding)
            fixResult = result
            fixingTask = nil

            // Clear the result after a short delay so it doesn't linger forever.
            try? await Task.sleep(nanoseconds: 4_000_000_000)
            if fixResult == result {
                fixResult = nil
            }
        }
    }

    @MainActor
    private func performFix(for finding: Finding) async -> String {
        let title = finding.title.lowercased()

        if finding.category == "system_maintenance" && title.contains("spotlight") {
            let result = await runProcess(["/usr/bin/mdutil", "-E", "/"])
            return result.success ? "Spotlight index reset." : "Spotlight reset failed: \(result.error)"
        }

        if title.contains("permissions") {
            return "Repair Disk Permissions is handled by macOS automatically since macOS El Capitan."
        }

        if title.contains("cache") || title.contains("caches") {
            let cachesURL = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Caches")
            NSWorkspace.shared.open(cachesURL)
            return "Opened Caches folder in Finder."
        }

        if title.contains("launch") || title.contains("agent") {
            if let path = finding.metadata["plist_path"]?.stringValue ?? finding.target.value as String? {
                revealInFinder(path: path)
                return "Revealed launch agent in Finder."
            }
            return "No launch agent path available."
        }

        if let url = URL(string: "x-apple.systempreferences:") {
            NSWorkspace.shared.open(url)
        }
        return "Opened System Preferences."
    }

    private func runProcess(_ args: [String]) async -> (success: Bool, error: String) {
        await withCheckedContinuation { continuation in
            let task = Process()
            task.executableURL = URL(fileURLWithPath: args[0])
            task.arguments = Array(args.dropFirst())

            let stderrPipe = Pipe()
            task.standardError = stderrPipe

            do {
                try task.run()
            } catch {
                continuation.resume(returning: (false, error.localizedDescription))
                return
            }

            DispatchQueue.global().async {
                let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                task.waitUntilExit()
                let stderr = String(data: stderrData, encoding: .utf8) ?? ""
                let success = task.terminationStatus == 0
                let message = stderr.isEmpty ? "Process exited with status \(task.terminationStatus)" : stderr
                continuation.resume(returning: (success, message))
            }
        }
    }
}

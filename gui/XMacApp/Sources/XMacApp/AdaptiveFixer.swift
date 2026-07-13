import Foundation
import Combine

extension Notification.Name {
    static let xmacOverrideBinaryPath = Notification.Name("XMacRunner.overrideBinaryPath")
    static let xmacGNNModelMissing = Notification.Name("XMacRunner.gnnModelMissing")
    static let xmacPermissionError = Notification.Name("XMacRunner.permissionError")
    static let xmacDiskFull = Notification.Name("XMacRunner.diskFull")
}

@MainActor
final class AdaptiveFixer: ObservableObject {
    static let shared = AdaptiveFixer()

    @Published var lastFix: FixResult?
    @Published var fixHistory: [FixResult] = []

    /// Pattern-matches the error to apply a known automatic fix without user input.
    func attemptFix(for error: Error, context: String) async -> FixResult {
        let fileManager = FileManager.default
        let description = error.localizedDescription.lowercased()
        let contextLower = context.lowercased()

        let result: FixResult

        if (description.contains("xmac") || contextLower.contains("xmac")) &&
            (description.contains("not found") || description.contains("no such file")) {
            let candidates = [
                "/opt/homebrew/bin/xmac",
                "/usr/local/bin/xmac",
                Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/xmac").path
            ]

            if let path = candidates.first(where: { fileManager.isExecutableFile(atPath: $0) }) {
                NotificationCenter.default.post(
                    name: .xmacOverrideBinaryPath,
                    object: nil,
                    userInfo: ["path": path]
                )
                result = FixResult(
                    id: UUID(),
                    date: Date(),
                    errorDescription: error.localizedDescription,
                    fixApplied: "Located xmac binary at \(path)",
                    success: true
                )
            } else {
                result = FixResult(
                    id: UUID(),
                    date: Date(),
                    errorDescription: error.localizedDescription,
                    fixApplied: "xmac binary not found in common paths",
                    success: false
                )
            }
        } else if description.contains("coreml") || description.contains("mlpackage") {
            let modelURL = Bundle.main.resourceURL?.appendingPathComponent("XMacGNN.mlpackage")
            if let modelURL = modelURL, !fileManager.fileExists(atPath: modelURL.path) {
                result = FixResult(
                    id: UUID(),
                    date: Date(),
                    errorDescription: error.localizedDescription,
                    fixApplied: "CoreML model missing — re-install app to restore XMacGNN.mlpackage",
                    success: false
                )
                NotificationCenter.default.post(name: .xmacGNNModelMissing, object: nil)
            } else {
                result = FixResult(
                    id: UUID(),
                    date: Date(),
                    errorDescription: error.localizedDescription,
                    fixApplied: "CoreML model appears present",
                    success: true
                )
            }
        } else if description.contains("permission") || description.contains("operation not permitted") {
            result = FixResult(
                id: UUID(),
                date: Date(),
                errorDescription: error.localizedDescription,
                fixApplied: "Skipped protected path — no action needed",
                success: true
            )
            NotificationCenter.default.post(
                name: .xmacPermissionError,
                object: nil,
                userInfo: ["context": context]
            )
        } else if description.contains("json") || description.contains("decoding") || description.contains("keynotfound") {
            let cacheDir = fileManager.homeDirectoryForCurrentUser
                .appendingPathComponent("Library/Caches/com.xmac.gui")
            if fileManager.fileExists(atPath: cacheDir.path) {
                try? fileManager.removeItem(at: cacheDir)
            }
            result = FixResult(
                id: UUID(),
                date: Date(),
                errorDescription: error.localizedDescription,
                fixApplied: "JSON parse error detected — clearing stale cache",
                success: true
            )
        } else if description.contains("scan was cancelled") {
            result = FixResult(
                id: UUID(),
                date: Date(),
                errorDescription: error.localizedDescription,
                fixApplied: "Scan cancelled by user — ready to retry",
                success: true
            )
        } else if description.contains("disk") && (description.contains("full") || description.contains("space")) {
            result = FixResult(
                id: UUID(),
                date: Date(),
                errorDescription: error.localizedDescription,
                fixApplied: "Disk full — cannot write Trash items. Free space manually.",
                success: false
            )
            NotificationCenter.default.post(name: .xmacDiskFull, object: nil)
        } else {
            result = FixResult(
                id: UUID(),
                date: Date(),
                errorDescription: error.localizedDescription,
                fixApplied: "Error logged for review — no automatic fix available",
                success: false
            )
        }

        lastFix = result
        if fixHistory.count >= 20 {
            fixHistory.removeFirst()
        }
        fixHistory.append(result)

        return result
    }

    /// Public hook to manually override the xmac binary path.
    func applyBinaryPathOverride(_ path: String) {
        NotificationCenter.default.post(
            name: .xmacOverrideBinaryPath,
            object: nil,
            userInfo: ["path": path]
        )
    }

    var statusSummary: String {
        let caught = fixHistory.count
        let fixed = fixHistory.filter(\.success).count
        return "\(caught) errors caught, \(fixed) fixed automatically"
    }
}

struct FixResult: Identifiable {
    let id: UUID
    let date: Date
    let errorDescription: String
    let fixApplied: String
    let success: Bool
}

import Foundation
import Combine

/// CrashReporter — passive error logger.
///
/// Records runtime errors to `~/Library/Logs/X-MaC/errors.log` and forwards
/// them to `AdaptiveFixer` for automatic recovery. Does **not** install
/// signal handlers — Apple's native crash reporter already captures
/// SIGSEGV/SIGABRT with full stack traces in ~/Library/Logs/DiagnosticReports/.
/// Installing custom signal handlers in a SwiftUI app is unsafe because
/// none of the Swift/ObjC runtime functions (FileManager, Date, String.write)
/// are async-signal-safe, causing secondary crashes.
@MainActor
final class CrashReporter: ObservableObject {
    static var shared = CrashReporter()

    @Published var recentErrors: [CrashEntry] = []

    private var logsDirectory: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Logs/X-MaC")
    }

    var logFileURL: URL {
        logsDirectory.appendingPathComponent("errors.log")
    }

    /// No-op. Apple's native crash reporter handles signals.
    /// Kept for backward compatibility with call sites.
    func install() {
        // Intentionally empty — see class docs.
    }

    /// Records an error in memory and on disk, then asks the adaptive fixer to attempt a repair.
    func record(error: Error, context: String) async {
        let entry = CrashEntry(
            id: UUID(),
            date: Date(),
            context: context,
            message: error.localizedDescription,
            wasFixed: false
        )

        if recentErrors.count >= 50 {
            recentErrors.removeFirst()
        }
        recentErrors.append(entry)
        writeEntry(entry)

        let fix = await AdaptiveFixer.shared.attemptFix(for: error, context: context)
        if fix.success, let index = recentErrors.firstIndex(where: { $0.id == entry.id }) {
            recentErrors[index].wasFixed = true
        }
    }

    /// Clears the in-memory error list and the on-disk error log.
    func clearLog() {
        recentErrors.removeAll()

        let fileManager = FileManager.default
        try? fileManager.removeItem(at: logFileURL)
    }

    private func writeEntry(_ entry: CrashEntry) {
        let fileManager = FileManager.default
        try? fileManager.createDirectory(at: logsDirectory, withIntermediateDirectories: true)

        let formatter = ISO8601DateFormatter()
        let line = "[\(formatter.string(from: entry.date))] [\(entry.context)] \(entry.message) | fixed=\(entry.wasFixed)\n"
        let data = Data(line.utf8)

        if fileManager.fileExists(atPath: logFileURL.path) {
            if let handle = try? FileHandle(forWritingTo: logFileURL) {
                handle.seekToEndOfFile()
                handle.write(data)
                handle.closeFile()
            }
        } else {
            try? line.write(to: logFileURL, atomically: true, encoding: .utf8)
        }
    }
}

struct CrashEntry: Identifiable {
    let id: UUID
    let date: Date
    let context: String
    let message: String
    var wasFixed: Bool
}

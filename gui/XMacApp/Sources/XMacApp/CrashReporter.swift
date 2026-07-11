import Foundation
import Combine

/// Shared context for the C signal handler so the crash log can include the last recorded error context.
nonisolated(unsafe) private var pendingCrashContext: String?

private let crashSignalHandler: @convention(c) (Int32) -> Void = { signal in
    let fileManager = FileManager.default
    let logsDir = fileManager.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/X-MaC")
    try? fileManager.createDirectory(at: logsDir, withIntermediateDirectories: true)

    let formatter = ISO8601DateFormatter()
    let timestamp = formatter.string(from: Date())
    let crashFile = logsDir.appendingPathComponent("crash_\(timestamp).log")

    var text = "Signal: \(signal)\n"
    text += "Date: \(timestamp)\n"
    if let context = pendingCrashContext {
        text += "Pending context: \(context)\n"
    }

    try? text.write(to: crashFile, atomically: true, encoding: .utf8)
    exit(signal)
}

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

    /// Installs SIGSEGV/SIGABRT handlers that write a crash log when the app crashes.
    func install() {
        signal(SIGSEGV, crashSignalHandler)
        signal(SIGABRT, crashSignalHandler)
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

        pendingCrashContext = context
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

        if let files = try? fileManager.contentsOfDirectory(
            at: logsDirectory,
            includingPropertiesForKeys: nil
        ) {
            for file in files where file.lastPathComponent.hasPrefix("crash_") {
                try? fileManager.removeItem(at: file)
            }
        }
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

import Foundation

struct ScanHistoryEntry: Identifiable, Codable {
    var id = UUID()
    let timestamp: Date
    let mode: String
    let findingCount: Int
    let reclaimableBytes: Int
    let durationSeconds: Double
}

@MainActor
final class ScanHistoryStore: ObservableObject {
    @Published var entries: [ScanHistoryEntry] = []
    private let savePath: URL

    init() {
        let home = FileManager.default.homeDirectoryForCurrentUser
        savePath = home
            .appendingPathComponent("Library/Caches/com.xmac.gui")
            .appendingPathComponent("scan_history.json")
        load()
    }

    func add(mode: String, findings: [Finding], duration: Double) {
        let reclaimable = findings.compactMap { $0.size_bytes }.reduce(0, +)
        let entry = ScanHistoryEntry(
            timestamp: Date(),
            mode: mode,
            findingCount: findings.count,
            reclaimableBytes: reclaimable,
            durationSeconds: duration
        )
        entries.append(entry)
        if entries.count > 100 {
            entries.removeFirst(entries.count - 100)
        }
        save()
    }
    
    func clear() {
        entries.removeAll()
        save()
    }

    private func load() {
        guard FileManager.default.fileExists(atPath: savePath.path),
              let data = try? Data(contentsOf: savePath),
              let decoded = try? JSONDecoder().decode([ScanHistoryEntry].self, from: data) else {
            return
        }
        entries = decoded
    }

    private func save() {
        try? FileManager.default.createDirectory(at: savePath.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(entries) {
            try? data.write(to: savePath)
        }
    }
}

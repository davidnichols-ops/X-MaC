import Foundation
import SwiftUI

// MARK: - Core Types

struct Finding: Identifiable, Codable, Hashable {
    let id: String
    let engine: String
    let severity: String
    let category: String
    let target: TargetInfo
    let title: String
    let description: String
    let metadata: [String: JSONValue]
    let discovered_at: TimeInfo
    let remediation_hint: String?
    let size_bytes: Int?
    let safety_rating: String?
    let safety_explanation: String?
    let safety_rule: String?
    let safety_confidence: Int?

    var displayName: String { title }
    var sizeFormatted: String {
        if let s = size_bytes, s > 0 { formatBytes(s) }
        else { "" }
    }
    var severityColor: Color {
        switch severity {
        case "info": return XTheme.info
        case "low": return XTheme.low
        case "medium": return XTheme.medium
        case "high": return XTheme.high
        case "critical": return XTheme.critical
        default: return XTheme.info
        }
    }
    var severityIcon: String {
        switch severity {
        case "info": return "info.circle"
        case "low": return "checkmark.circle"
        case "medium": return "exclamationmark.triangle"
        case "high": return "exclamationmark.octagon"
        case "critical": return "xmark.octagon"
        default: return "info.circle"
        }
    }
    var safetyColor: Color {
        switch safety_rating {
        case "safe": return XTheme.safe
        case "review": return XTheme.medium
        case "protected": return XTheme.danger
        default: return XTheme.textSecondary
        }
    }
    var safetyIcon: String {
        switch safety_rating {
        case "safe": return "checkmark.shield.fill"
        case "review": return "exclamationmark.shield.fill"
        case "protected": return "xmark.shield.fill"
        default: return "shield"
        }
    }
    var hasSafetyInfo: Bool { safety_rating != nil }
}

struct TargetInfo: Codable, Hashable {
    let type: String
    let value: String

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        type = try container.decode(String.self, forKey: .type)
        // Accept both string and numeric values
        if let s = try? container.decode(String.self, forKey: .value) {
            value = s
        } else if let i = try? container.decode(Int.self, forKey: .value) {
            value = "\(i)"
        } else if let d = try? container.decode(Double.self, forKey: .value) {
            value = "\(d)"
        } else {
            value = ""
        }
    }
}

struct TimeInfo: Codable, Hashable {
    let secs_since_epoch: Int
    let nanos_since_epoch: Int
}

// MARK: - JSON Value (for flexible metadata)

enum JSONValue: Codable, Hashable {
    case string(String)
    case int(Int)
    case double(Double)
    case bool(Bool)
    case array([JSONValue])
    case object([String: JSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if let v = try? container.decode(String.self) { self = .string(v) }
        else if let v = try? container.decode(Int.self) { self = .int(v) }
        else if let v = try? container.decode(Double.self) { self = .double(v) }
        else if let v = try? container.decode(Bool.self) { self = .bool(v) }
        else if let v = try? container.decode([JSONValue].self) { self = .array(v) }
        else if let v = try? container.decode([String: JSONValue].self) { self = .object(v) }
        else { self = .null }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .string(let v): try container.encode(v)
        case .int(let v): try container.encode(v)
        case .double(let v): try container.encode(v)
        case .bool(let v): try container.encode(v)
        case .array(let v): try container.encode(v)
        case .object(let v): try container.encode(v)
        case .null: try container.encodeNil()
        }
    }

    var stringValue: String? {
        if case .string(let s) = self { return s }
        return nil
    }
    var intValue: Int? {
        if case .int(let i) = self { return i }
        if case .double(let d) = self { return Int(d) }
        return nil
    }
    var doubleValue: Double? {
        if case .double(let d) = self { return d }
        if case .int(let i) = self { return Double(i) }
        return nil
    }
    var boolValue: Bool? {
        if case .bool(let b) = self { return b }
        return nil
    }
}

// MARK: - GNN Response

struct GNNResponse: Codable {
    let scores: [GNNScore]
    let summary: GNNSummary?
    let purge_plan: PurgePlan?
}

struct GNNScore: Codable, Identifiable, Hashable {
    let path: String
    let label: String
    let safety_score: Double
    let anomaly_score: Double
    let confidence: Double?
    let explanation: String?
    let size_bytes: Int?
    var id: String { path }

    var safetyColor: Color {
        if safety_score >= 0.7 { return XTheme.safe }
        else if safety_score >= 0.4 { return XTheme.warning }
        else { return XTheme.danger }
    }
    var safetyIcon: String {
        if safety_score >= 0.7 { return "checkmark.circle.fill" }
        else if safety_score >= 0.4 { return "questionmark.circle.fill" }
        else { return "xmark.circle.fill" }
    }
    var sizeFormatted: String {
        if let s = size_bytes, s > 0 { formatBytes(s) }
        else { "" }
    }
    var confidenceText: String {
        if let c = confidence { return String(format: "Confidence: %.0f%%", c * 100) }
        return "Confidence: N/A"
    }
    var explanationText: String {
        explanation ?? "No explanation available."
    }
}

struct GNNSummary: Codable {
    let total_files: Int
    let safe_files: Int
    let review_files: Int
    let danger_files: Int
    let avg_safety: Double
    let avg_anomaly: Double
    let potential_reclaim_bytes: Int?
}

struct PurgePlan: Codable {
    let impact_weighted_order: [PurgeItem]
    let anomaly_hotspots: [AnomalyHotspot]
    let cleanup_confidence: String
    let cross_directory_patterns: [CrossDirPattern]
}

struct PurgeItem: Codable, Identifiable, Hashable {
    let path: String
    let safety_score: Double
    let size_bytes: Int
    let impact_score: Double
    var id: String { path }
    var sizeFormatted: String { formatBytes(size_bytes) }
}

struct AnomalyHotspot: Codable, Identifiable, Hashable {
    let directory: String
    let anomaly_count: Int
    let avg_anomaly: Double
    var id: String { directory }
}

struct CrossDirPattern: Codable, Identifiable, Hashable {
    let pattern: String
    let file_count: Int
    let total_size: Int
    let example_paths: [String]
    var id: String { pattern }
    var sizeFormatted: String { formatBytes(total_size) }
}

// MARK: - Helpers

func formatBytes(_ bytes: Int) -> String {
    // Use binary units (1 GB = 1024³) to match macOS Activity Monitor
    let units = [(1024 * 1024 * 1024 * 1024, "TB"),
                 (1024 * 1024 * 1024, "GB"),
                 (1024 * 1024, "MB"),
                 (1024, "KB")]
    for (divisor, suffix) in units {
        if bytes >= divisor {
            let value = Double(bytes) / Double(divisor)
            if value >= 100 {
                return String(format: "%.0f %@", value, suffix)
            } else if value >= 10 {
                return String(format: "%.1f %@", value, suffix)
            } else {
                return String(format: "%.2f %@", value, suffix)
            }
        }
    }
    return "\(bytes) B"
}

func formatBytes(_ bytes: UInt64) -> String {
    formatBytes(Int(bytes))
}

// MARK: - FilePathDisplay

struct FilePathDisplay: View {
    let path: String
    var fontSize: CGFloat = 12
    var pathFontSize: CGFloat = 10
    var showIcon: Bool = true
    var maxPathLines: Int = 2

    private var fileName: String {
        (path as NSString).lastPathComponent
    }

    private var directoryPath: String {
        let nsPath = path as NSString
        let dir = nsPath.deletingLastPathComponent
        return dir.isEmpty ? "/" : dir
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 6) {
                if showIcon {
                    Image(systemName: iconName)
                        .font(.system(size: fontSize - 1))
                        .foregroundStyle(XTheme.textTertiary)
                }
                Text(fileName)
                    .font(.system(size: fontSize, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            Button {
                revealInFinder(path: path)
            } label: {
                HStack(spacing: 4) {
                    Image(systemName: "folder")
                        .font(.system(size: pathFontSize - 2))
                        .foregroundStyle(XTheme.accent.opacity(0.7))
                    Text(path)
                        .font(.system(size: pathFontSize, design: .monospaced))
                        .foregroundStyle(XTheme.accent.opacity(0.8))
                        .lineLimit(maxPathLines)
                        .truncationMode(.middle)
                        .underline()
                }
            }
            .buttonStyle(.plain)
            .help("Click to reveal in Finder")
        }
    }

    private var iconName: String {
        let ext = (path as NSString).pathExtension.lowercased()
        switch ext {
        case "app": return "app"
        case "pyc", "py": return "python"
        case "rs": return "rust"
        case "swift": return "swift"
        case "js", "ts": return "javascript"
        case "json", "yaml", "yml", "toml", "plist", "xml": return "gearshape"
        case "png", "jpg", "jpeg", "gif", "svg", "webp": return "photo"
        case "mp4", "mov", "avi", "mkv": return "video"
        case "mp3", "wav", "flac", "aac": return "music.note"
        case "pdf": return "doc.richtext"
        case "log": return "doc.text"
        case "tmp", "temp": return "clock"
        case "dmg", "iso": return "opticaldisc"
        case "":
            var isDir: ObjCBool = false
            if FileManager.default.fileExists(atPath: path, isDirectory: &isDir), isDir.boolValue {
                return "folder"
            }
            return "doc"
        default: return "doc"
        }
    }
}

func revealInFinder(path: String) {
    let url = URL(fileURLWithPath: path)
    let fm = FileManager.default
    if fm.fileExists(atPath: path) {
        NSWorkspace.shared.activateFileViewerSelecting([url])
    } else if fm.fileExists(atPath: url.deletingLastPathComponent().path) {
        NSWorkspace.shared.open(url.deletingLastPathComponent())
    }
}

// MARK: - Zen Mode Result

struct ZenResult: Codable {
    let duration_secs: Double
    let health_before: Double
    let health_after: Double
    let memory_before: ZenMemorySummary
    let memory_after: ZenMemorySummary
    let reclaimable_bytes: UInt64
    let reclaimed_bytes: UInt64
    let findings_count: Int
    let maintenance_tasks_run: Int
    let top_categories: [ZenCategoryEntry]
    let steps: [ZenStep]
}

struct ZenMemorySummary: Codable {
    let total_bytes: UInt64
    let used_bytes: UInt64
    let free_bytes: UInt64
    let utilization: Double
}

struct ZenCategoryEntry: Codable, Identifiable {
    let _0: String
    let _1: UInt64
    var id: String { _0 }
    var category: String { _0 }
    var size: UInt64 { _1 }

    init(from decoder: Decoder) throws {
        var container = try decoder.unkeyedContainer()
        _0 = try container.decode(String.self)
        _1 = try container.decode(UInt64.self)
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(_0)
        try container.encode(_1)
    }
}

struct ZenStep: Codable, Identifiable {
    let name: String
    let status: String
    let detail: String
    var id: String { name }
}

// MARK: - AI Advisor Recommendation

struct AdvisorRecommendation: Codable, Identifiable, Hashable {
    let id: String
    let severity: String
    let category: String
    let title: String
    let explanation: String
    let action: String
    let command: String?
    let estimated_impact: String
    let confidence: Double
    let auto_safe: Bool

    var severityColor: Color {
        switch severity {
        case "critical": return XTheme.critical
        case "high": return XTheme.high
        case "medium": return XTheme.medium
        case "low": return XTheme.low
        case "info": return XTheme.info
        default: return XTheme.info
        }
    }

    var severityIcon: String {
        switch severity {
        case "critical": return "xmark.octagon.fill"
        case "high": return "exclamationmark.octagon.fill"
        case "medium": return "exclamationmark.triangle.fill"
        case "low": return "checkmark.circle.fill"
        case "info": return "info.circle.fill"
        default: return "info.circle.fill"
        }
    }
}

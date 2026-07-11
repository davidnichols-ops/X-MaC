import SwiftUI
import UniformTypeIdentifiers

struct XMacReportDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.plainText, .json] }
    static var writableContentTypes: [UTType] { [.plainText, .json] }

    var findings: [Finding]
    var gnnScores: [GNNScore]

    init(findings: [Finding], gnnScores: [GNNScore]) {
        self.findings = findings
        self.gnnScores = gnnScores
    }

    init(configuration: ReadConfiguration) throws {
        findings = []
        gnnScores = []
    }

    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        var lines: [String] = []
        lines.append("# X-MaC Cleanup Report")
        lines.append("Generated: \(Date().description)")
        lines.append("Findings: \(findings.count)")
        lines.append("")

        let grouped = Dictionary(grouping: findings, by: { $0.category })
            .sorted { $0.value.compactMap { $0.size_bytes }.reduce(0, +) > $1.value.compactMap { $0.size_bytes }.reduce(0, +) }

        for (category, items) in grouped {
            let total = items.compactMap { $0.size_bytes }.reduce(0, +)
            lines.append("## \(category.replacingOccurrences(of: "_", with: " ").capitalized) — \(formatBytes(total))")
            for item in items {
                lines.append("- [\(item.severity)] \(item.title) — \(item.target.value) — \(formatBytes(item.size_bytes ?? 0))")
            }
            lines.append("")
        }

        if !gnnScores.isEmpty {
            lines.append("## Neural Scores")
            for score in gnnScores {
                lines.append("- \(score.path): safety \(String(format: "%.2f", score.safety_score)), anomaly \(String(format: "%.2f", score.anomaly_score))")
            }
        }

        let data = lines.joined(separator: "\n").data(using: .utf8) ?? Data()
        return FileWrapper(regularFileWithContents: data)
    }
}

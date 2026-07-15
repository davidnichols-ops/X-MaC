import SwiftUI

struct SelectableFindingRow: View {
    @EnvironmentObject var runner: XMacRunner
    let finding: Finding

    private var path: String { finding.target.value }
    private var isSelected: Bool { runner.selectedPaths.contains(path) }

    var body: some View {
        Button {
            runner.toggleSelection(path: path)
        } label: {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 16))
                    .foregroundStyle(isSelected ? XTheme.safe : XTheme.textTertiary)
                    .frame(width: 20)
                Image(systemName: finding.severityIcon)
                    .font(.system(size: 15, weight: .medium))
                    .foregroundStyle(finding.severityColor)
                    .frame(width: 20)
                VStack(alignment: .leading, spacing: 3) {
                    HStack {
                        Text(finding.title)
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                            .lineLimit(1)
                        Spacer()
                        if !finding.sizeFormatted.isEmpty {
                            Text(finding.sizeFormatted)
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                    }
                    FilePathDisplay(path: path, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                    Text(finding.description)
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textSecondary)
                        .lineLimit(2)
                    if finding.hasSafetyInfo {
                        SafetyBadge(finding: finding)
                    }
                }
            }
            .padding(.vertical, 5)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!isSafePath(path) || runner.isScanning)
        .help(isSafePath(path) ? "Select for review and move to Trash" : "Protected path")
    }

    private func isSafePath(_ path: String) -> Bool {
        let url = URL(fileURLWithPath: path).standardizedFileURL
        let home = FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL
        let temp = URL(fileURLWithPath: "/private/tmp").standardizedFileURL
        return url.path.hasPrefix(home.path + "/") || url.path.hasPrefix(temp.path + "/")
    }
}

// MARK: - Safety Badge

struct SafetyBadge: View {
    let finding: Finding

    private var ratingText: String {
        switch finding.safety_rating {
        case "safe": return "Safe"
        case "review": return "Review"
        case "protected": return "Protected"
        default: return finding.safety_rating ?? "Unknown"
        }
    }

    private var tooltip: String {
        var parts: [String] = []
        if let explanation = finding.safety_explanation, !explanation.isEmpty {
            parts.append(explanation)
        }
        if let rule = finding.safety_rule, !rule.isEmpty {
            parts.append("Rule: \(rule)")
        }
        if let confidence = finding.safety_confidence {
            parts.append("Confidence: \(confidence)%")
        }
        return parts.isEmpty ? ratingText : parts.joined(separator: " • ")
    }

    var body: some View {
        HStack(spacing: 4) {
            Image(systemName: finding.safetyIcon)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(finding.safetyColor)
            Text(ratingText)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(finding.safetyColor)
            if let confidence = finding.safety_confidence {
                Text("\(confidence)%")
                    .font(.system(size: 9, weight: .medium))
                    .foregroundStyle(XTheme.textTertiary)
            }
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(finding.safetyColor.opacity(0.12))
        .clipShape(RoundedRectangle(cornerRadius: 4))
        .help(tooltip)
    }
}

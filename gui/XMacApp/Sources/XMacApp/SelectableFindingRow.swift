import SwiftUI

// MARK: - SelectableFindingRow
//
// Displays a finding with a selection checkbox.
// Protected items are shown as locked and cannot be selected.
// Review items are selectable but show a warning badge.

struct SelectableFindingRow: View {
    @EnvironmentObject var runner: XMacRunner
    let finding: Finding

    private var path: String { finding.target.value }
    private var isSelected: Bool { runner.selectedPaths.contains(path) }

    private var safetyRating: String {
        finding.safety_rating ?? "unclassified"
    }

    private var isProtected: Bool {
        safetyRating.lowercased() == "protected"
    }

    private var isPathSafe: Bool {
        runner.isSafeCleanupPath(path)
    }

    private var canSelect: Bool {
        !isProtected && isPathSafe && !runner.isScanning
    }

    var body: some View {
        if isProtected {
            // Protected items show as locked, non-interactive
            protectedRow
        } else {
            selectableRow
        }
    }

    // MARK: - Selectable Row

    private var selectableRow: some View {
        Button {
            runner.toggleSelection(path: path)
        } label: {
            HStack(alignment: .top, spacing: XTheme.spacing.sm + 2) {
                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 16))
                    .foregroundStyle(isSelected ? XTheme.color.statusSafe : XTheme.color.textTertiary)
                    .frame(width: 20)

                Image(systemName: finding.severityIcon)
                    .font(.system(size: 15, weight: .medium))
                    .foregroundStyle(finding.severityColor)
                    .frame(width: 20)

                VStack(alignment: .leading, spacing: 3) {
                    HStack {
                        Text(finding.title)
                            .font(XTheme.typography.bodySemibold)
                            .foregroundStyle(XTheme.color.textPrimary)
                            .lineLimit(1)
                        Spacer()
                        if !finding.sizeFormatted.isEmpty {
                            Text(finding.sizeFormatted)
                                .font(XTheme.typography.captionMedium)
                                .foregroundStyle(XTheme.color.textSecondary)
                        }
                    }
                    FilePathDisplay(path: path, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                    Text(finding.description)
                        .font(XTheme.typography.monoSmall)
                        .foregroundStyle(XTheme.color.textSecondary)
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
        .disabled(!canSelect)
        .help(canSelect ? "Select for review and move to Trash" : "Cannot select this item")
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(finding.title), \(finding.severity), \(finding.sizeFormatted)")
        .accessibilityValue(isSelected ? "selected" : "not selected")
        .accessibilityHint(canSelect ? "Toggle selection for cleanup" : "Cannot be selected for cleanup")
    }

    // MARK: - Protected Row

    private var protectedRow: some View {
        HStack(alignment: .top, spacing: XTheme.spacing.sm + 2) {
            Image(systemName: "lock.fill")
                .font(.system(size: 14))
                .foregroundStyle(XTheme.color.statusDanger)
                .frame(width: 20)

            Image(systemName: finding.severityIcon)
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(finding.severityColor)
                .frame(width: 20)

            VStack(alignment: .leading, spacing: 3) {
                HStack {
                    Text(finding.title)
                        .font(XTheme.typography.bodySemibold)
                        .foregroundStyle(XTheme.color.textPrimary)
                        .lineLimit(1)
                    Spacer()
                    if !finding.sizeFormatted.isEmpty {
                        Text(finding.sizeFormatted)
                            .font(XTheme.typography.captionMedium)
                            .foregroundStyle(XTheme.color.textSecondary)
                    }
                }
                FilePathDisplay(path: path, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                Text(finding.description)
                    .font(XTheme.typography.monoSmall)
                    .foregroundStyle(XTheme.color.textSecondary)
                    .lineLimit(2)
                if finding.hasSafetyInfo {
                    SafetyBadge(finding: finding)
                }
            }
        }
        .padding(.vertical, 5)
        .opacity(0.7)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(finding.title), protected, \(finding.sizeFormatted)")
        .accessibilityHint("This item is protected and cannot be selected for cleanup")
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
        HStack(spacing: XTheme.spacing.xxs) {
            Image(systemName: finding.safetyIcon)
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(finding.safetyColor)
            Text(ratingText)
                .font(XTheme.typography.microMedium)
                .foregroundStyle(finding.safetyColor)
            if let confidence = finding.safety_confidence {
                Text("\(confidence)%")
                    .font(XTheme.typography.monoSmall)
                    .foregroundStyle(XTheme.color.textTertiary)
            }
        }
        .padding(.horizontal, XTheme.spacing.xs + 2)
        .padding(.vertical, XTheme.spacing.xxs)
        .background(finding.safetyColor.opacity(0.12))
        .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.badge))
        .help(tooltip)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Safety: \(ratingText)\(finding.safety_confidence.map { ", confidence \($0) percent" } ?? "")")
    }
}

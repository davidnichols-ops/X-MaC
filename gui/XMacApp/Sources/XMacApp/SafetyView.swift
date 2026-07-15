import Foundation
import SwiftUI

// MARK: - Safety Models
// Swift Codable models matching the Rust JSON output from
// `xmac safety --action list` and `xmac safety --action classify --path <path>`.

struct SafetyRulesResponse: Codable {
    let total_rules: Int
    let counts_by_rating: [String: Int]
    let rules: [SafetyRuleInfo]
}

struct SafetyRuleInfo: Codable, Identifiable {
    var id: String { name }
    let name: String
    let description: String
    let rating: String
    let paths: [String]
    let confidence: Int
    let category: String?
    let upstream_commit: String?
}

struct PathClassification: Codable {
    let path: String
    let rating: String
    let rule: String?
    let description: String?
    let confidence: Int?
    let category: String?
    let preselected: Bool?
    let explanation: String?
}

// MARK: - Safety View

struct SafetyView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var pathInput: String = ""

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                header

                if runner.safetyRulesLoading {
                    loadingView
                } else if let rules = runner.safetyRules {
                    rulesContent(rules)
                } else if let err = runner.error {
                    errorView(err)
                } else {
                    placeholderView
                }

                classifierSection
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
        .onAppear {
            if runner.safetyRules == nil && !runner.safetyRulesLoading {
                runner.loadSafetyRules()
            }
        }
    }

    // MARK: - Header

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "shield.lefthalf.filled")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.accent)
                .xGlow(XTheme.accent, radius: 6)
            VStack(alignment: .leading, spacing: 4) {
                Text("Safety")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("GNN-powered path classification and safety rules")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()

            Button {
                runner.loadSafetyRules()
            } label: {
                HStack(spacing: 6) {
                    Image(systemName: "arrow.clockwise")
                        .font(.system(size: 12, weight: .medium))
                    Text("Refresh")
                        .font(.system(size: 12, weight: .medium))
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 7)
                .background(XTheme.accent.opacity(0.15))
                .foregroundStyle(XTheme.accent)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(XTheme.accent.opacity(0.3), lineWidth: 1)
                )
            }
            .buttonStyle(.plain)
            .disabled(runner.safetyRulesLoading)
        }
    }

    // MARK: - Loading

    private var loadingView: some View {
        XCard {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.2)
                    .tint(XTheme.accent)
                Text("Loading safety rules...")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity, minHeight: 200)
        }
    }

    // MARK: - Error

    private func errorView(_ message: String) -> some View {
        XCard {
            VStack(spacing: 12) {
                Image(systemName: "xmark.octagon.fill")
                    .font(.system(size: 36, weight: .light))
                    .foregroundStyle(XTheme.danger)
                Text("Failed to load safety rules")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(message)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
                    .multilineTextAlignment(.center)
                Button("Try Again") {
                    runner.loadSafetyRules()
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
            }
            .frame(maxWidth: .infinity, minHeight: 180)
        }
    }

    // MARK: - Placeholder

    private var placeholderView: some View {
        XCard {
            VStack(spacing: 12) {
                Image(systemName: "shield.lefthalf.filled")
                    .font(.system(size: 36, weight: .light))
                    .foregroundStyle(XTheme.accent)
                Text("No safety rules loaded")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
                Button("Load Rules") {
                    runner.loadSafetyRules()
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
            }
            .frame(maxWidth: .infinity, minHeight: 200)
        }
    }

    // MARK: - Rules Content

    private func rulesContent(_ rules: SafetyRulesResponse) -> some View {
        VStack(spacing: 16) {
            summaryCard(rules)

            let safeRules = rules.rules.filter { $0.rating.lowercased() == "safe" }
            let reviewRules = rules.rules.filter { $0.rating.lowercased() == "review" }
            let protectedRules = rules.rules.filter { $0.rating.lowercased() == "protected" }

            if !safeRules.isEmpty {
                rulesSection(title: "Safe", icon: "checkmark.seal.fill", rules: safeRules, color: XTheme.safe)
            }
            if !reviewRules.isEmpty {
                rulesSection(title: "Review", icon: "exclamationmark.triangle.fill", rules: reviewRules, color: XTheme.medium)
            }
            if !protectedRules.isEmpty {
                rulesSection(title: "Protected", icon: "lock.shield.fill", rules: protectedRules, color: XTheme.danger)
            }
        }
    }

    // MARK: - Summary Card

    private func summaryCard(_ rules: SafetyRulesResponse) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 8) {
                    Image(systemName: "chart.bar.doc.horizontal")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.accent)
                        .xGlow(XTheme.accent, radius: 4)
                    Text("Summary")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(XTheme.textPrimary)
                    Spacer()
                    Text("\(rules.total_rules) total rules")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(XTheme.accent)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(XTheme.accent.opacity(0.12))
                        .clipShape(Capsule())
                }

                Divider().background(XTheme.cardBorder)

                HStack(spacing: 24) {
                    summaryStat(
                        label: "Safe",
                        value: "\(rules.counts_by_rating["safe"] ?? 0)",
                        icon: "checkmark.seal.fill",
                        color: XTheme.safe
                    )
                    summaryStat(
                        label: "Review",
                        value: "\(rules.counts_by_rating["review"] ?? 0)",
                        icon: "exclamationmark.triangle.fill",
                        color: XTheme.medium
                    )
                    summaryStat(
                        label: "Protected",
                        value: "\(rules.counts_by_rating["protected"] ?? 0)",
                        icon: "lock.shield.fill",
                        color: XTheme.danger
                    )
                }
            }
        }
    }

    private func summaryStat(label: String, value: String, icon: String, color: Color) -> some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .font(.system(size: 16))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 18, weight: .bold, design: .rounded))
                .foregroundStyle(XTheme.textPrimary)
            Text(label)
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity)
    }

    // MARK: - Rules Section

    private func rulesSection(title: String, icon: String, rules: [SafetyRuleInfo], color: Color) -> some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "\(title) Rules", icon: icon, count: rules.count)

                ForEach(rules) { rule in
                    ruleRow(rule, color: color)
                }
            }
        }
    }

    private func ruleRow(_ rule: SafetyRuleInfo, color: Color) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                Image(systemName: "circle.fill")
                    .font(.system(size: 8))
                    .foregroundStyle(color)
                    .xGlow(color, radius: 3)
                VStack(alignment: .leading, spacing: 2) {
                    Text(rule.name)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(XTheme.textPrimary)
                    Text(rule.description)
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                        .lineLimit(2)
                }
                Spacer()
                VStack(alignment: .trailing, spacing: 2) {
                    Text("\(rule.confidence)%")
                        .font(.system(size: 12, weight: .semibold, design: .rounded))
                        .foregroundStyle(color)
                    if let category = rule.category, !category.isEmpty {
                        Text(category)
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                }
            }

            if !rule.paths.isEmpty {
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(rule.paths, id: \.self) { p in
                        HStack(spacing: 4) {
                            Image(systemName: "folder")
                                .font(.system(size: 9))
                                .foregroundStyle(XTheme.textTertiary)
                            Text(p)
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundStyle(XTheme.textTertiary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                    }
                }
                .padding(.leading, 18)
            }
        }
        .padding(.vertical, 4)
    }

    // MARK: - Path Classifier

    private var classifierSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Path Classifier", icon: "magnifyingglass.circle.fill")

                Text("Enter a filesystem path to classify its safety rating.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)

                HStack(spacing: 10) {
                    TextField("/path/to/file/or/directory", text: $pathInput)
                        .textFieldStyle(.plain)
                        .font(.system(size: 12, design: .monospaced))
                        .padding(.horizontal, 10)
                        .padding(.vertical, 8)
                        .background(XTheme.bgTertiary)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(XTheme.cardBorder, lineWidth: 1)
                        )

                    Button {
                        classifyCurrentPath()
                    } label: {
                        HStack(spacing: 6) {
                            if runner.pathClassificationLoading {
                                ProgressView()
                                    .scaleEffect(0.7)
                                    .tint(XTheme.accent)
                            } else {
                                Image(systemName: "tag.fill")
                                    .font(.system(size: 12, weight: .medium))
                            }
                            Text("Classify")
                                .font(.system(size: 12, weight: .medium))
                        }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 8)
                        .background(XTheme.accent.opacity(0.15))
                        .foregroundStyle(XTheme.accent)
                        .clipShape(RoundedRectangle(cornerRadius: 8))
                        .overlay(
                            RoundedRectangle(cornerRadius: 8)
                                .stroke(XTheme.accent.opacity(0.3), lineWidth: 1)
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(pathInput.isEmpty || runner.pathClassificationLoading)
                }

                if let classification = runner.pathClassification {
                    classificationResult(classification)
                }
            }
        }
    }

    private func classificationResult(_ c: PathClassification) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Divider().background(XTheme.cardBorder)

            HStack(spacing: 10) {
                Image(systemName: ratingIcon(c.rating))
                    .font(.system(size: 20))
                    .foregroundStyle(ratingColor(c.rating))
                    .xGlow(ratingColor(c.rating), radius: 4)
                VStack(alignment: .leading, spacing: 2) {
                    Text(c.rating.capitalized)
                        .font(.system(size: 16, weight: .bold, design: .rounded))
                        .foregroundStyle(ratingColor(c.rating))
                    Text(c.path)
                        .font(.system(size: 10, design: .monospaced))
                        .foregroundStyle(XTheme.textTertiary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                Spacer()
                if let confidence = c.confidence {
                    VStack(spacing: 2) {
                        Text("\(confidence)%")
                            .font(.system(size: 14, weight: .bold, design: .rounded))
                            .foregroundStyle(ratingColor(c.rating))
                        Text("confidence")
                            .font(.system(size: 9))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                }
            }

            if let rule = c.rule, !rule.isEmpty {
                classificationDetail(label: "Rule", value: rule)
            }
            if let desc = c.description, !desc.isEmpty {
                classificationDetail(label: "Description", value: desc)
            }
            if let category = c.category, !category.isEmpty {
                classificationDetail(label: "Category", value: category)
            }
            if let preselected = c.preselected {
                classificationDetail(label: "Preselected", value: preselected ? "Yes" : "No")
            }
            if let explanation = c.explanation, !explanation.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Explanation")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(XTheme.textSecondary)
                    Text(explanation)
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textPrimary)
                        .lineLimit(5)
                }
            }
        }
    }

    private func classificationDetail(label: String, value: String) -> some View {
        HStack(spacing: 8) {
            Text(label)
                .font(.system(size: 11, weight: .semibold))
                .foregroundStyle(XTheme.textSecondary)
            Spacer()
            Text(value)
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textPrimary)
        }
    }

    // MARK: - Helpers

    private func classifyCurrentPath() {
        let trimmed = pathInput.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        runner.classifyPath(trimmed)
    }

    private func ratingColor(_ rating: String) -> Color {
        switch rating.lowercased() {
        case "safe": return XTheme.safe
        case "review": return XTheme.medium
        case "protected": return XTheme.danger
        default: return XTheme.textTertiary
        }
    }

    private func ratingIcon(_ rating: String) -> String {
        switch rating.lowercased() {
        case "safe": return "checkmark.seal.fill"
        case "review": return "exclamationmark.triangle.fill"
        case "protected": return "lock.shield.fill"
        default: return "questionmark.circle.fill"
        }
    }
}

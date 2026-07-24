import SwiftUI

// MARK: - Depth View (Ops 1-30: filesystem integrity, permissions, symlinks)

struct DepthView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        scanDetailView("Filesystem Integrity", icon: "checkmark.shield.fill", findings: runner.depthFindings, runAction: { runner.runDepthScan() })
    }
}

// MARK: - Quick Scan View (Composite: clean + maintain + disk)

struct QuickScanView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        scanDetailView("Quick Scan", icon: "bolt.fill", findings: runner.quickFindings, runAction: { runner.runQuickScan() })
    }
}

// MARK: - Purge View (Ops 601-630: cleanup execution, undo, safety)

struct PurgeView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var purgeOutput = ""

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                header
                purgeWarning
                if !purgeOutput.isEmpty { outputView }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }

    private var header: some View {
        HStack(spacing: 12) {
            Image(systemName: "trash.fill")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.danger)
            VStack(alignment: .leading, spacing: 4) {
                Text("Purge — Cleanup Execution")
                    .font(.system(size: 24, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)
                Text("Execute reviewed cleanup plans (ops 601-630)")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }

    private var purgeWarning: some View {
        XCard {
            VStack(spacing: 12) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 28))
                    .foregroundStyle(XTheme.danger)
                Text("Purge moves files to Trash")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text("This is the only CLI subcommand that modifies the filesystem. It uses trash-first with undo support and full safety validation.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
                    .multilineTextAlignment(.center)

                Button("Generate Fix Script (Safe)") {
                    Task {
                        let result = try? await runner.runProcess([
                            runner.xmacPath, "--format", "json", "clean", "--fix-script", "/tmp/xmac_fix_script.sh"
                        ])
                        if let (stdout, _) = result {
                            purgeOutput = stdout
                        }
                    }
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
            }
            .frame(maxWidth: .infinity)
        }
    }

    private var outputView: some View {
        XCard {
            VStack(alignment: .leading, spacing: 8) {
                Text("Output")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                ScrollView {
                    Text(purgeOutput)
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(XTheme.textSecondary)
                        .textSelection(.enabled)
                }
                .frame(maxHeight: 300)
            }
        }
    }
}

// MARK: - Shared Scan Detail View

func scanDetailView(_ title: String, icon: String, findings: [Finding], runAction: @escaping () -> Void) -> some View {
    ScrollView {
        VStack(spacing: 16) {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .font(.system(size: 24, weight: .bold))
                    .foregroundStyle(XTheme.accent)
                    .xGlow(XTheme.accent, radius: 6)
                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.metallicGradient)
                }
                Spacer()
                Button("Scan") { runAction() }
                    .buttonStyle(.borderedProminent)
                    .tint(XTheme.accent)
            }
            .padding(.top, 8)

            if findings.isEmpty {
                XCard {
                    VStack(spacing: 12) {
                        Image(systemName: icon)
                            .font(.system(size: 36, weight: .light))
                            .foregroundStyle(XTheme.accent)
                        Text("No results yet")
                            .font(.system(size: 14))
                            .foregroundStyle(XTheme.textSecondary)
                        Text("Click Scan to run \(title)")
                            .font(.system(size: 12))
                            .foregroundStyle(XTheme.textTertiary)
                        Button("Scan Now") { runAction() }
                            .buttonStyle(.borderedProminent)
                            .tint(XTheme.accent)
                    }
                    .frame(maxWidth: .infinity, minHeight: 200)
                }
            } else {
                // Summary
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Results", icon: "list.bullet.rectangle", count: findings.count)
                        let bySeverity = Dictionary(grouping: findings, by: { $0.severity })
                        HStack(spacing: 12) {
                            ForEach(["critical", "high", "medium", "low", "info"], id: \.self) { sev in
                                if let count = bySeverity[sev]?.count {
                                    VStack(spacing: 2) {
                                        Text("\(count)")
                                            .font(.system(size: 16, weight: .bold, design: .rounded))
                                            .foregroundStyle(severityColor(sev))
                                        Text(sev.capitalized)
                                            .font(.system(size: 10))
                                            .foregroundStyle(XTheme.textTertiary)
                                    }
                                }
                            }
                        }
                    }
                }

                // Findings list
                XCard {
                    VStack(alignment: .leading, spacing: 6) {
                        ForEach(findings) { finding in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack(spacing: 8) {
                                    Image(systemName: severityIcon(finding.severity))
                                        .foregroundStyle(severityColor(finding.severity))
                                        .font(.system(size: 11))
                                    Text(finding.title)
                                        .font(.system(size: 12, weight: .medium))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Spacer()
                                    Text(finding.category)
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.textTertiary)
                                }
                                Text(finding.description)
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                    .lineLimit(2)
                                if let sz = finding.size_bytes, sz > 0 {
                                    Text(formatBytes(sz))
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.accent)
                                }
                            }
                            .padding(.vertical, 4)
                            Divider().background(XTheme.cardBorder)
                        }
                    }
                }
            }
        }
        .padding(20)
    }
    .background(XTheme.voidGradient)
}

// MARK: - Helpers

func severityIcon(_ severity: String) -> String {
    switch severity.lowercased() {
    case "critical": return "xmark.octagon.fill"
    case "high": return "exclamationmark.octagon.fill"
    case "medium": return "exclamationmark.triangle.fill"
    case "low": return "checkmark.circle.fill"
    case "info": return "info.circle.fill"
    default: return "circle.fill"
    }
}

import SwiftUI

struct AdvisorView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                heroSection

                if runner.isAdvisorRunning {
                    advisorRunningSection
                } else if !runner.advisorRecommendations.isEmpty {
                    advisorResultsSection
                } else {
                    advisorIdleSection
                }
            }
            .padding(28)
        }
        .background(XTheme.voidGradient)
    }

    // MARK: - Hero

    private var heroSection: some View {
        VStack(spacing: 16) {
            VStack(spacing: 6) {
                Image(systemName: "brain.head.profile")
                    .font(.system(size: 48, weight: .light))
                    .foregroundStyle(XTheme.neuralGradient)
                    .xHeroGlow(XTheme.anomaly, radius: 16)

                Text("AI Advisor")
                    .font(.system(size: 32, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.metallicGradient)

                Text("Intelligent recommendations in plain English")
                    .font(.system(size: 14))
                    .foregroundStyle(XTheme.textSecondary)
            }

            Button {
                runner.runAdvisor()
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "sparkles")
                        .font(.system(size: 16, weight: .semibold))
                    Text("Analyze System")
                        .font(.system(size: 16, weight: .bold))
                }
                .frame(width: 220, height: 52)
                .foregroundStyle(.white)
                .background(XTheme.neuralGradient)
                .clipShape(Capsule())
            }
            .buttonStyle(.plain)
            .xHeroGlow(XTheme.anomaly, radius: 14)
            .disabled(runner.isAdvisorRunning)
        }
        .frame(maxWidth: .infinity)
        .padding(.top, 16)
    }

    // MARK: - Running

    private var advisorRunningSection: some View {
        XCard {
            VStack(spacing: 16) {
                ProgressView()
                    .scaleEffect(1.2)
                Text("Analyzing system health...")
                    .font(.system(size: 16, weight: .medium))
                    .foregroundStyle(XTheme.textPrimary)
                Text("Collecting memory, CPU, disk, thermal, and battery telemetry")
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
        }
    }

    // MARK: - Idle

    private var advisorIdleSection: some View {
        XCard {
            VStack(alignment: .leading, spacing: 16) {
                XSectionHeader(title: "How the AI Advisor works", icon: "brain")

                AdvisorFeatureRow(icon: "gauge", title: "Multi-dimensional Analysis",
                    description: "Examines memory, CPU, disk, thermal, and battery health simultaneously")
                AdvisorFeatureRow(icon: "text.bubble", title: "Plain English Explanations",
                    description: "Explains what's happening and why it matters in language anyone can understand")
                AdvisorFeatureRow(icon: "arrow.right.circle", title: "Actionable Recommendations",
                    description: "Each recommendation includes a specific action and CLI command to run")
                AdvisorFeatureRow(icon: "chart.line.uptrend.xyaxis", title: "Adaptive Learning",
                    description: "Learns from your feedback to improve recommendation quality over time")
                AdvisorFeatureRow(icon: "clock.badge.exclamationmark", title: "Proactive Warnings",
                    description: "Predicts future pressure before it becomes critical")
            }
        }
    }

    // MARK: - Results

    private var advisorResultsSection: some View {
        VStack(spacing: 16) {
            // Health score card
            XCard {
                VStack(spacing: 12) {
                    HStack(spacing: 16) {
                        ZStack {
                            Circle()
                                .stroke(XTheme.cardBorder, lineWidth: 8)
                                .frame(width: 80, height: 80)
                            Circle()
                                .trim(from: 0, to: runner.advisorHealthScore / 100.0)
                                .stroke(healthColor(runner.advisorHealthScore), style: StrokeStyle(lineWidth: 8, lineCap: .round))
                                .frame(width: 80, height: 80)
                                .rotationEffect(.degrees(-90))
                            VStack(spacing: 0) {
                                Text("\(Int(runner.advisorHealthScore))")
                                    .font(.system(size: 24, weight: .bold, design: .rounded))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text("health")
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("System Status")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                            Text(runner.advisorStatus.capitalized)
                                .font(.system(size: 20, weight: .bold, design: .rounded))
                                .foregroundStyle(healthColor(runner.advisorHealthScore))
                            Text("\(runner.advisorRecommendations.count) recommendations")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textTertiary)
                        }

                        Spacer()
                    }
                }
            }

            // Recommendations list
            ForEach(runner.advisorRecommendations) { rec in
                AdvisorRecommendationCard(recommendation: rec)
            }
        }
    }

    // MARK: - Helpers

    private func healthColor(_ score: Double) -> Color {
        if score >= 85 { return XTheme.safe }
        if score >= 70 { return XTheme.accent }
        if score >= 50 { return XTheme.medium }
        if score >= 30 { return XTheme.high }
        return XTheme.critical
    }
}

// MARK: - Recommendation Card

private struct AdvisorRecommendationCard: View {
    let recommendation: AdvisorRecommendation

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                HStack(spacing: 10) {
                    Image(systemName: recommendation.severityIcon)
                        .font(.system(size: 18))
                        .foregroundStyle(recommendation.severityColor)
                        .xGlow(recommendation.severityColor, radius: 4)

                    VStack(alignment: .leading, spacing: 2) {
                        Text(recommendation.title)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                        Text("[\(recommendation.severity.uppercased())] \(recommendation.category.capitalized)")
                            .font(.system(size: 10, weight: .medium, design: .monospaced))
                            .foregroundStyle(recommendation.severityColor)
                    }

                    Spacer()

                    // Confidence gauge
                    VStack(spacing: 2) {
                        Text("\(Int(recommendation.confidence * 100))%")
                            .font(.system(size: 14, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.textPrimary)
                        Text("confidence")
                            .font(.system(size: 9))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                }

                Text(recommendation.explanation)
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textSecondary)
                    .lineLimit(4)

                HStack(spacing: 8) {
                    Image(systemName: "arrow.right.circle.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(XTheme.accent)
                    Text(recommendation.action)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(XTheme.textPrimary)
                    Spacer()
                }

                if let cmd = recommendation.command {
                    HStack(spacing: 6) {
                        Image(systemName: "terminal")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textTertiary)
                        Text(cmd)
                            .font(.system(size: 11, design: .monospaced))
                            .foregroundStyle(XTheme.accent.opacity(0.8))
                            .lineLimit(1)
                            .truncationMode(.tail)
                        Spacer()
                    }
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(XTheme.bgVoid.opacity(0.5))
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                }

                HStack(spacing: 6) {
                    Image(systemName: "chart.bar")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textTertiary)
                    Text(recommendation.estimated_impact)
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textTertiary)
                    Spacer()
                    if recommendation.auto_safe {
                        HStack(spacing: 3) {
                            Image(systemName: "checkmark.shield.fill")
                                .font(.system(size: 10))
                            Text("Safe to auto-execute")
                                .font(.system(size: 10, weight: .medium))
                        }
                        .foregroundStyle(XTheme.safe)
                    }
                }
            }
        }
    }
}

private struct AdvisorFeatureRow: View {
    let icon: String
    let title: String
    let description: String

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 18))
                .foregroundStyle(XTheme.anomaly)
                .frame(width: 24)
                .xGlow(XTheme.anomaly, radius: 3)
            VStack(alignment: .leading, spacing: 3) {
                Text(title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(XTheme.textPrimary)
                Text(description)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
            }
            Spacer()
        }
    }
}

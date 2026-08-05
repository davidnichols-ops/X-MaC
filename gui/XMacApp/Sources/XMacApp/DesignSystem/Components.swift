import SwiftUI

// MARK: - XButton
//
// Primary button with neural gradient. Use for main calls to action.
// Supports reduced motion, accessibility, disabled state, and keyboard focus.

struct XButton: View {
    let label: String
    let icon: String?
    let action: () -> Void
    var disabled: Bool = false
    var loading: Bool = false

    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    init(_ label: String, icon: String? = nil, disabled: Bool = false, loading: Bool = false, action: @escaping () -> Void) {
        self.label = label
        self.icon = icon
        self.disabled = disabled
        self.loading = loading
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack(spacing: XTheme.spacing.sm) {
                if loading {
                    ProgressView()
                        .controlSize(.small)
                        .tint(.white)
                } else if let icon {
                    Image(systemName: icon)
                        .font(XTheme.typography.bodyMedium)
                }
                Text(label)
                    .font(XTheme.typography.bodySemibold)
            }
            .foregroundStyle(.white)
            .padding(.horizontal, XTheme.spacing.lg)
            .frame(minHeight: 36)
            .background(
                Group {
                    if disabled || loading {
                        XTheme.color.accentDim
                    } else {
                        XTheme.gradient.neural
                    }
                }
            )
            .clipShape(Capsule())
            .if(!disabled && !loading) { $0.xGlow(XTheme.color.accent, radius: 6) }
            .opacity(disabled ? 0.6 : 1.0)
        }
        .buttonStyle(.plain)
        .disabled(disabled || loading)
        .accessibilityLabel(label)
        .accessibilityHint(disabled ? "Currently disabled" : "")
    }
}

// MARK: - XSecondaryButton
//
// Bordered button for secondary actions.

struct XSecondaryButton: View {
    let label: String
    let icon: String?
    let action: () -> Void
    var disabled: Bool = false

    init(_ label: String, icon: String? = nil, disabled: Bool = false, action: @escaping () -> Void) {
        self.label = label
        self.icon = icon
        self.disabled = disabled
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack(spacing: XTheme.spacing.sm) {
                if let icon {
                    Image(systemName: icon)
                        .font(XTheme.typography.bodyMedium)
                }
                Text(label)
                    .font(XTheme.typography.bodyMedium)
            }
            .foregroundStyle(XTheme.color.textPrimary)
            .padding(.horizontal, XTheme.spacing.md)
            .frame(minHeight: 32)
            .background(XTheme.color.backgroundTertiary)
            .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.button))
            .overlay(
                RoundedRectangle(cornerRadius: XTheme.radius.button)
                    .stroke(XTheme.color.cardBorder, lineWidth: XTheme.border.standard)
            )
            .opacity(disabled ? 0.5 : 1.0)
        }
        .buttonStyle(.plain)
        .disabled(disabled)
        .accessibilityLabel(label)
    }
}

// MARK: - XDestructiveButton
//
// Button for destructive actions. Always paired with confirmation.

struct XDestructiveButton: View {
    let label: String
    let icon: String?
    let action: () -> Void
    var disabled: Bool = false
    var loading: Bool = false

    init(_ label: String, icon: String? = nil, disabled: Bool = false, loading: Bool = false, action: @escaping () -> Void) {
        self.label = label
        self.icon = icon
        self.disabled = disabled
        self.loading = loading
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack(spacing: XTheme.spacing.sm) {
                if loading {
                    ProgressView()
                        .controlSize(.small)
                        .tint(XTheme.color.statusDanger)
                } else if let icon {
                    Image(systemName: icon)
                        .font(XTheme.typography.bodyMedium)
                }
                Text(label)
                    .font(XTheme.typography.bodyMedium)
            }
            .foregroundStyle(XTheme.color.statusDanger)
            .padding(.horizontal, XTheme.spacing.md)
            .frame(minHeight: 32)
            .background(XTheme.color.statusDanger.opacity(0.1))
            .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.button))
            .overlay(
                RoundedRectangle(cornerRadius: XTheme.radius.button)
                    .stroke(XTheme.color.statusDanger.opacity(0.3), lineWidth: XTheme.border.standard)
            )
            .opacity(disabled ? 0.5 : 1.0)
        }
        .buttonStyle(.plain)
        .disabled(disabled || loading)
        .accessibilityLabel(label)
    }
}

// MARK: - XBadge
//
// Small status badge with icon, text, and color.
// Never used as the sole indicator — always paired with text.

struct XBadge: View {
    let text: String
    let icon: String?
    let color: Color

    init(_ text: String, icon: String? = nil, color: Color = XTheme.color.accent) {
        self.text = text
        self.icon = icon
        self.color = color
    }

    var body: some View {
        HStack(spacing: XTheme.spacing.xxs) {
            if let icon {
                Image(systemName: icon)
                    .font(.system(size: 9, weight: .medium))
            }
            Text(text)
                .font(XTheme.typography.microMedium)
        }
        .foregroundStyle(color)
        .padding(.horizontal, XTheme.spacing.xs + 2)
        .padding(.vertical, XTheme.spacing.xxs)
        .background(color.opacity(0.12))
        .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.badge))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(text)
    }
}

// MARK: - XStatusBadge
//
// Safety/severity badge that always includes icon + text + color.

struct XStatusBadge: View {
    let rating: String
    let confidence: Int?

    init(rating: String, confidence: Int? = nil) {
        self.rating = rating
        self.confidence = confidence
    }

    private var statusColor: Color {
        XTheme.color.safety(rating)
    }

    private var statusIcon: String {
        switch rating.lowercased() {
        case "safe": return "checkmark.shield.fill"
        case "review": return "eye.fill"
        case "protected", "danger": return "exclamationmark.shield.fill"
        default: return "questionmark.circle"
        }
    }

    var body: some View {
        HStack(spacing: XTheme.spacing.xxs) {
            Image(systemName: statusIcon)
                .font(.system(size: 9, weight: .medium))
            Text(rating.capitalized)
                .font(XTheme.typography.microMedium)
            if let c = confidence {
                Text("\(c)%")
                    .font(XTheme.typography.monoSmall)
                    .foregroundStyle(XTheme.color.textTertiary)
            }
        }
        .foregroundStyle(statusColor)
        .padding(.horizontal, XTheme.spacing.xs + 2)
        .padding(.vertical, XTheme.spacing.xxs)
        .background(statusColor.opacity(0.12))
        .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.badge))
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Safety: \(rating)\(confidence.map { ", confidence \($0) percent" } ?? "")")
    }
}

// MARK: - XMetric
//
// Compact metric display: icon + value + label.

struct XMetric: View {
    let icon: String
    let value: String
    let label: String
    var color: Color = XTheme.color.accent

    var body: some View {
        VStack(spacing: XTheme.spacing.xs) {
            Image(systemName: icon)
                .font(.system(size: 18, weight: .medium))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 18, weight: .bold, design: .rounded))
                .foregroundStyle(XTheme.color.textPrimary)
            Text(label)
                .font(XTheme.typography.micro)
                .foregroundStyle(XTheme.color.textTertiary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, XTheme.spacing.sm)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(label): \(value)")
    }
}

// MARK: - XEmptyState
//
// Standard empty state with icon, title, message, and optional action.

struct XEmptyState: View {
    let icon: String
    let title: String
    let message: String
    var actionLabel: String?
    var action: (() -> Void)?

    var body: some View {
        VStack(spacing: XTheme.spacing.md) {
            Image(systemName: icon)
                .font(.system(size: 48, weight: .medium))
                .foregroundStyle(XTheme.color.textTertiary)
            Text(title)
                .font(XTheme.typography.heroSubtitle)
                .foregroundStyle(XTheme.color.textPrimary)
            Text(message)
                .font(XTheme.typography.body)
                .foregroundStyle(XTheme.color.textSecondary)
                .multilineTextAlignment(.center)
            if let actionLabel, let action {
                XSecondaryButton(actionLabel, action: action)
                    .padding(.top, XTheme.spacing.xs)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(XTheme.spacing.xxl)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title). \(message)")
    }
}

// MARK: - XErrorState
//
// Standard error state with summary, details disclosure, retry, and copy diagnostics.

struct XErrorState: View {
    let summary: String
    let detail: String?
    var retryAction: (() -> Void)?

    @State private var showDetails = false

    var body: some View {
        VStack(spacing: XTheme.spacing.md) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 36, weight: .medium))
                .foregroundStyle(XTheme.color.statusDanger)

            Text(summary)
                .font(XTheme.typography.bodySemibold)
                .foregroundStyle(XTheme.color.textPrimary)
                .multilineTextAlignment(.center)

            if let detail {
                DisclosureGroup("Details", isExpanded: $showDetails) {
                    Text(detail)
                        .font(XTheme.typography.mono)
                        .foregroundStyle(XTheme.color.textSecondary)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(XTheme.spacing.sm)
                        .background(XTheme.color.backgroundTertiary)
                        .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.sm))
                }
                .font(XTheme.typography.caption)
                .foregroundStyle(XTheme.color.textSecondary)
            }

            HStack(spacing: XTheme.spacing.md) {
                if let retryAction {
                    XSecondaryButton("Retry", icon: "arrow.clockwise", action: retryAction)
                }
                if let detail {
                    Button("Copy Diagnostics") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString("\(summary)\n\n\(detail)", forType: .string)
                    }
                    .font(XTheme.typography.caption)
                    .foregroundStyle(XTheme.color.textSecondary)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(XTheme.spacing.xxl)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Error: \(summary)")
    }
}

// MARK: - XLoadingState
//
// Loading state with phase, progress, elapsed time, and cancel.

struct XLoadingState: View {
    let phase: String
    var progress: Double?
    var elapsed: TimeInterval?
    var cancelAction: (() -> Void)?

    var body: some View {
        VStack(spacing: XTheme.spacing.lg) {
            if let progress {
                VStack(spacing: XTheme.spacing.sm) {
                    ProgressView(value: progress)
                        .progressViewStyle(.linear)
                        .tint(XTheme.color.accent)
                    Text("\(Int(progress * 100))%")
                        .font(XTheme.typography.monoSmall)
                        .foregroundStyle(XTheme.color.textTertiary)
                }
                .frame(maxWidth: 240)
            } else {
                ProgressView()
                    .controlSize(.large)
                    .tint(XTheme.color.accent)
            }

            Text(phase)
                .font(XTheme.typography.bodyMedium)
                .foregroundStyle(XTheme.color.textSecondary)

            if let elapsed {
                Text(formatElapsed(elapsed))
                    .font(XTheme.typography.monoSmall)
                    .foregroundStyle(XTheme.color.textTertiary)
            }

            if let cancelAction {
                XSecondaryButton("Cancel", icon: "xmark", action: cancelAction)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(XTheme.spacing.xxl)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Loading: \(phase)")
    }

    private func formatElapsed(_ seconds: TimeInterval) -> String {
        if seconds < 60 {
            return String(format: "%.1fs", seconds)
        }
        let mins = Int(seconds / 60)
        let secs = Int(seconds.truncatingRemainder(dividingBy: 60))
        return String(format: "%dm %ds", mins, secs)
    }
}

// MARK: - XSearchField
//
// Reusable search field with clear button and accessibility.

struct XSearchField: View {
    @Binding var text: String
    var placeholder: String = "Search…"

    var body: some View {
        HStack(spacing: XTheme.spacing.sm) {
            Image(systemName: "magnifyingglass")
                .font(.system(size: 12))
                .foregroundStyle(XTheme.color.textTertiary)
                .accessibilityLabel("Search")

            TextField(placeholder, text: $text)
                .font(XTheme.typography.body)
                .textFieldStyle(.plain)
                .accessibilityLabel(placeholder)

            if !text.isEmpty {
                Button {
                    text = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.color.textTertiary)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Clear search")
            }
        }
        .padding(XTheme.spacing.sm + 2)
        .background(XTheme.color.backgroundTertiary)
        .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.md))
    }
}

// MARK: - XDisclosureRow
//
// Expandable row for "Why am I seeing this?" and similar disclosures.

struct XDisclosureRow<Label: View, Content: View>: View {
    let label: Label
    let content: Content
    @State private var expanded = false

    init(@ViewBuilder label: () -> Label, @ViewBuilder content: () -> Content) {
        self.label = label()
        self.content = content()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button {
                withAnimation(.easeInOut(duration: 0.2)) {
                    expanded.toggle()
                }
            } label: {
                HStack {
                    label
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 10, weight: .medium))
                        .foregroundStyle(XTheme.color.textTertiary)
                        .rotationEffect(.degrees(expanded ? 90 : 0))
                }
            }
            .buttonStyle(.plain)

            if expanded {
                content
                    .padding(.top, XTheme.spacing.sm)
                    .transition(.opacity)
            }
        }
    }
}

import SwiftUI

enum XTheme {
    // Background
    static let bgPrimary = Color(red: 0.06, green: 0.07, blue: 0.09)
    static let bgSecondary = Color(red: 0.09, green: 0.10, blue: 0.13)
    static let bgTertiary = Color(red: 0.12, green: 0.13, blue: 0.17)

    // Accent
    static let accent = Color(red: 0.20, green: 0.60, blue: 1.0)
    static let accentDim = Color(red: 0.15, green: 0.40, blue: 0.70)

    // Severity
    static let info = Color(red: 0.40, green: 0.60, blue: 0.90)
    static let low = Color(red: 0.30, green: 0.75, blue: 0.40)
    static let medium = Color(red: 0.95, green: 0.75, blue: 0.25)
    static let high = Color(red: 0.95, green: 0.45, blue: 0.20)
    static let critical = Color(red: 0.90, green: 0.25, blue: 0.25)

    // GNN
    static let safe = Color(red: 0.30, green: 0.80, blue: 0.40)
    static let warning = Color(red: 0.95, green: 0.75, blue: 0.25)
    static let danger = Color(red: 0.90, green: 0.30, blue: 0.30)
    static let anomaly = Color(red: 0.65, green: 0.35, blue: 0.90)

    // Text
    static let textPrimary = Color.white
    static let textSecondary = Color(white: 0.65)
    static let textTertiary = Color(white: 0.45)

    // Card
    static let cardBg = Color(red: 0.10, green: 0.11, blue: 0.14)
    static let cardBorder = Color(white: 0.12)
}

struct XCard<Content: View>: View {
    let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        content
            .padding(16)
            .background(XTheme.cardBg)
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(XTheme.cardBorder, lineWidth: 1)
            )
    }
}

struct XSectionHeader: View {
    let title: String
    let icon: String
    var count: Int? = nil

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .foregroundStyle(XTheme.accent)
                .font(.system(size: 14, weight: .semibold))
            Text(title)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(XTheme.textPrimary)
            if let c = count {
                Text("\(c)")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textTertiary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(XTheme.bgTertiary)
                    .clipShape(Capsule())
            }
            Spacer()
        }
    }
}

extension View {
    func xGlow(_ color: Color = XTheme.accent, radius: CGFloat = 8) -> some View {
        self.shadow(color: color.opacity(0.3), radius: radius)
    }
}

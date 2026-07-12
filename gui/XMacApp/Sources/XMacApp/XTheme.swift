import SwiftUI

// MARK: - XTheme
// Color palette distilled from the X-MaC app icon:
//   • Deep void blacks with navy undertones
//   • Electric cyan HUD accents (#57CAFF)
//   • Metallic silver for primary text and the X mark
//   • Teal depths for secondary surfaces
//   • Neural network glow aesthetic throughout

enum XTheme {
    // ── Backgrounds (from icon's dominant dark tones) ────────────────────
    /// Deepest void — #000810 — the abyss behind the neural globe
    static let bgVoid = Color(red: 0.00, green: 0.03, blue: 0.06)
    /// Primary background — #001020 — navy abyss
    static let bgPrimary = Color(red: 0.00, green: 0.06, blue: 0.13)
    /// Secondary surface — #0A1525 — sidebar and panels
    static let bgSecondary = Color(red: 0.04, green: 0.08, blue: 0.15)
    /// Tertiary surface — #102030 — cards and raised surfaces
    static let bgTertiary = Color(red: 0.06, green: 0.13, blue: 0.19)

    // ── Accents (from icon's HUD readout cyan) ───────────────────────────
    /// Electric cyan — #57CAFF — primary accent (HUD glow, neural connections)
    static let accent = Color(red: 0.34, green: 0.79, blue: 1.0)
    /// Bright cyan — #3FB8FF — hover/active states
    static let accentBright = Color(red: 0.25, green: 0.72, blue: 1.0)
    /// Dim cyan — #1A6090 — inactive/disabled accent
    static let accentDim = Color(red: 0.10, green: 0.38, blue: 0.56)
    /// Deep neural blue — #1A2F4E — dark accent for backgrounds
    static let neuralBlue = Color(red: 0.10, green: 0.18, blue: 0.31)

    // ── Metallic (from the X mark in the icon) ───────────────────────────
    /// Metallic silver — #E8E8E8 — primary text, the X
    static let metallic = Color(red: 0.91, green: 0.91, blue: 0.91)
    /// Warm metallic — #F0E7DF — gradient endpoint for metallic text
    static let metallicWarm = Color(red: 0.94, green: 0.91, blue: 0.87)
    /// Brushed steel — #BFBFBE — secondary metallic
    static let steel = Color(red: 0.75, green: 0.75, blue: 0.75)

    // ── Teal depths (from icon's mid-tones) ──────────────────────────────
    /// Mid teal — #4288A8 — secondary accent
    static let teal = Color(red: 0.26, green: 0.53, blue: 0.66)
    /// Deep teal — #29546B — dark accent
    static let tealDeep = Color(red: 0.16, green: 0.33, blue: 0.42)

    // ── Severity (tuned to match the icon's palette) ─────────────────────
    static let info = Color(red: 0.34, green: 0.79, blue: 1.0)    // cyan
    static let low = Color(red: 0.30, green: 0.80, blue: 0.55)    // teal-green
    static let medium = Color(red: 0.95, green: 0.75, blue: 0.25) // amber
    static let high = Color(red: 0.95, green: 0.50, blue: 0.25)   // orange
    static let critical = Color(red: 0.90, green: 0.28, blue: 0.32) // red

    // ── GNN safety (neural network colored) ──────────────────────────────
    static let safe = Color(red: 0.20, green: 0.82, blue: 0.52)   // neon green
    static let warning = Color(red: 0.95, green: 0.75, blue: 0.25) // amber
    static let danger = Color(red: 0.90, green: 0.30, blue: 0.35)  // red
    static let anomaly = Color(red: 0.55, green: 0.40, blue: 0.95) // neural purple

    // ── Text ─────────────────────────────────────────────────────────────
    /// Primary text — metallic silver
    static let textPrimary = Color(red: 0.91, green: 0.91, blue: 0.91)
    /// Secondary text — dimmed steel
    static let textSecondary = Color(red: 0.58, green: 0.62, blue: 0.68)
    /// Tertiary text — deep steel
    static let textTertiary = Color(red: 0.38, green: 0.42, blue: 0.48)

    // ── Cards ────────────────────────────────────────────────────────────
    /// Card background — dark navy with slight blue tint
    static let cardBg = Color(red: 0.05, green: 0.10, blue: 0.16)
    /// Card border — subtle cyan-tinted border
    static let cardBorder = Color(red: 0.12, green: 0.20, blue: 0.28)

    // ── Gradients (matching the icon's visual identity) ──────────────────
    /// Neural gradient — cyan to teal (for buttons, active states)
    static var neuralGradient: LinearGradient {
        LinearGradient(
            colors: [accent, teal],
            startPoint: .leading,
            endPoint: .trailing
        )
    }

    /// HUD gradient — bright cyan to deep blue (for hero elements)
    static var hudGradient: LinearGradient {
        LinearGradient(
            colors: [accentBright, Color(red: 0.10, green: 0.40, blue: 0.80)],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }

    /// Metallic gradient — silver to warm steel (for text, the X mark)
    static var metallicGradient: LinearGradient {
        LinearGradient(
            colors: [metallic, metallicWarm],
            startPoint: .top,
            endPoint: .bottom
        )
    }

    /// Void gradient — deepest black to navy (for backgrounds)
    static var voidGradient: LinearGradient {
        LinearGradient(
            colors: [bgVoid, bgPrimary],
            startPoint: .top,
            endPoint: .bottom
        )
    }

    /// Card gradient — subtle navy gradient for raised surfaces
    static var cardGradient: LinearGradient {
        LinearGradient(
            colors: [cardBg, bgSecondary],
            startPoint: .top,
            endPoint: .bottom
        )
    }

    /// Sidebar gradient — deep neural blue to void
    static var sidebarGradient: LinearGradient {
        LinearGradient(
            colors: [bgSecondary, bgVoid],
            startPoint: .top,
            endPoint: .bottom
        )
    }
}

// MARK: - XCard

struct XCard<Content: View>: View {
    let content: Content
    var glow: Bool = true

    init(glow: Bool = true, @ViewBuilder content: () -> Content) {
        self.glow = glow
        self.content = content()
    }

    var body: some View {
        content
            .padding(16)
            .background(XTheme.cardGradient)
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(XTheme.cardBorder, lineWidth: 1)
            )
            .shadow(color: XTheme.accent.opacity(glow ? 0.04 : 0), radius: 6)
    }
}

// MARK: - XSectionHeader

struct XSectionHeader: View {
    let title: String
    let icon: String
    var count: Int? = nil

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: icon)
                .foregroundStyle(XTheme.accent)
                .font(.system(size: 14, weight: .semibold))
                .xGlow(XTheme.accent, radius: 4)
            Text(title)
                .font(.system(size: 14, weight: .semibold))
                .foregroundStyle(XTheme.textPrimary)
            if let c = count {
                Text("\(c)")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.accent)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(XTheme.accent.opacity(0.12))
                    .clipShape(Capsule())
            }
            Spacer()
        }
    }
}

// MARK: - Glow extension

extension View {
    /// Applies a cyan neural-network glow shadow to the view.
    func xGlow(_ color: Color = XTheme.accent, radius: CGFloat = 8) -> some View {
        self.shadow(color: color.opacity(0.35), radius: radius)
    }

    /// Stronger glow for hero elements.
    func xHeroGlow(_ color: Color = XTheme.accent, radius: CGFloat = 14) -> some View {
        self.shadow(color: color.opacity(0.4), radius: radius, x: 0, y: 2)
    }

    /// Metallic text style — applies the metallic gradient as foreground.
    func xMetallicText() -> some View {
        self
            .foregroundStyle(XTheme.metallicGradient)
    }
}

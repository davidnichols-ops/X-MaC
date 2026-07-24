import SwiftUI
import AppKit

// MARK: - XTheme Design Token System
//
// A complete token system for X-MaC, organized into semantic namespaces.
// Colors are adaptive (dark/light) to support both appearances.
//
// Usage in views:
//   XTheme.color.backgroundPrimary
//   XTheme.color.textSecondary
//   XTheme.color.statusSafe
//   XTheme.spacing.cardPadding
//   XTheme.radius.card
//   XTheme.typography.body
//
// Legacy direct access (XTheme.accent, XTheme.bgVoid, etc.) remains available
// as computed aliases for backward compatibility during migration.

enum XTheme {

    // MARK: - Color Tokens

    enum color {
        // ── Backgrounds ───────────────────────────────────────────────────
        /// Deepest background — used for window canvas
        static let backgroundCanvas = Color.adaptive(
            dark: Color(red: 0.00, green: 0.03, blue: 0.06),
            light: Color(red: 0.96, green: 0.96, blue: 0.97)
        )
        /// Primary background — main content area
        static let backgroundPrimary = Color.adaptive(
            dark: Color(red: 0.00, green: 0.06, blue: 0.13),
            light: Color(red: 0.98, green: 0.98, blue: 0.99)
        )
        /// Secondary surface — sidebar, panels
        static let backgroundSecondary = Color.adaptive(
            dark: Color(red: 0.04, green: 0.08, blue: 0.15),
            light: Color(red: 0.94, green: 0.94, blue: 0.96)
        )
        /// Tertiary surface — cards, raised surfaces
        static let backgroundTertiary = Color.adaptive(
            dark: Color(red: 0.06, green: 0.13, blue: 0.19),
            light: Color(red: 1.0, green: 1.0, blue: 1.0)
        )

        // ── Accents ───────────────────────────────────────────────────────
        /// Primary accent — electric cyan
        static let accent = Color.adaptive(
            dark: Color(red: 0.34, green: 0.79, blue: 1.0),
            light: Color(red: 0.20, green: 0.60, blue: 0.90)
        )
        /// Bright accent — hover/active states
        static let accentBright = Color.adaptive(
            dark: Color(red: 0.25, green: 0.72, blue: 1.0),
            light: Color(red: 0.15, green: 0.55, blue: 0.85)
        )
        /// Dim accent — inactive/disabled
        static let accentDim = Color.adaptive(
            dark: Color(red: 0.10, green: 0.38, blue: 0.56),
            light: Color(red: 0.60, green: 0.75, blue: 0.85)
        )
        /// Neural blue — dark accent for AI backgrounds
        static let neuralBlue = Color.adaptive(
            dark: Color(red: 0.10, green: 0.18, blue: 0.31),
            light: Color(red: 0.85, green: 0.92, blue: 0.98)
        )

        // ── Text ──────────────────────────────────────────────────────────
        static let textPrimary = Color.adaptive(
            dark: Color(red: 0.91, green: 0.91, blue: 0.91),
            light: Color(red: 0.12, green: 0.12, blue: 0.13)
        )
        static let textSecondary = Color.adaptive(
            dark: Color(red: 0.58, green: 0.62, blue: 0.68),
            light: Color(red: 0.40, green: 0.40, blue: 0.42)
        )
        static let textTertiary = Color.adaptive(
            dark: Color(red: 0.38, green: 0.42, blue: 0.48),
            light: Color(red: 0.60, green: 0.60, blue: 0.62)
        )

        // ── Surfaces ──────────────────────────────────────────────────────
        static let cardBackground = Color.adaptive(
            dark: Color(red: 0.05, green: 0.10, blue: 0.16),
            light: Color(red: 1.0, green: 1.0, blue: 1.0)
        )
        static let cardBorder = Color.adaptive(
            dark: Color(red: 0.12, green: 0.20, blue: 0.28),
            light: Color(red: 0.88, green: 0.88, blue: 0.90)
        )
        static let cardBackgroundElevated = Color.adaptive(
            dark: Color(red: 0.08, green: 0.15, blue: 0.22),
            light: Color(red: 0.99, green: 0.99, blue: 1.0)
        )

        // ── Severity (findings, alerts) ───────────────────────────────────
        static let severityInfo = accent
        static let severityLow = Color.adaptive(
            dark: Color(red: 0.30, green: 0.80, blue: 0.55),
            light: Color(red: 0.15, green: 0.60, blue: 0.40)
        )
        static let severityMedium = Color.adaptive(
            dark: Color(red: 0.95, green: 0.75, blue: 0.25),
            light: Color(red: 0.75, green: 0.55, blue: 0.10)
        )
        static let severityHigh = Color.adaptive(
            dark: Color(red: 0.95, green: 0.50, blue: 0.25),
            light: Color(red: 0.80, green: 0.35, blue: 0.10)
        )
        static let severityCritical = Color.adaptive(
            dark: Color(red: 0.90, green: 0.28, blue: 0.32),
            light: Color(red: 0.75, green: 0.15, blue: 0.18)
        )

        // ── Safety (GNN, rule engine) ─────────────────────────────────────
        static let statusSafe = Color.adaptive(
            dark: Color(red: 0.20, green: 0.82, blue: 0.52),
            light: Color(red: 0.10, green: 0.60, blue: 0.38)
        )
        static let statusWarning = severityMedium
        static let statusDanger = Color.adaptive(
            dark: Color(red: 0.90, green: 0.30, blue: 0.35),
            light: Color(red: 0.75, green: 0.15, blue: 0.20)
        )
        static let statusAnomaly = Color.adaptive(
            dark: Color(red: 0.55, green: 0.40, blue: 0.95),
            light: Color(red: 0.40, green: 0.25, blue: 0.75)
        )
        static let statusProtected = statusDanger
        static let statusReview = statusWarning

        // ── Metallic (logo, hero text) ────────────────────────────────────
        static let metallic = Color.adaptive(
            dark: Color(red: 0.91, green: 0.91, blue: 0.91),
            light: Color(red: 0.20, green: 0.20, blue: 0.22)
        )
        static let metallicWarm = Color.adaptive(
            dark: Color(red: 0.94, green: 0.91, blue: 0.87),
            light: Color(red: 0.25, green: 0.23, blue: 0.20)
        )
        static let steel = Color.adaptive(
            dark: Color(red: 0.75, green: 0.75, blue: 0.75),
            light: Color(red: 0.45, green: 0.45, blue: 0.47)
        )

        // ── Teal ──────────────────────────────────────────────────────────
        static let teal = Color.adaptive(
            dark: Color(red: 0.26, green: 0.53, blue: 0.66),
            light: Color(red: 0.15, green: 0.40, blue: 0.52)
        )
        static let tealDeep = Color.adaptive(
            dark: Color(red: 0.16, green: 0.33, blue: 0.42),
            light: Color(red: 0.30, green: 0.50, blue: 0.58)
        )

        // ── Semantic helpers ──────────────────────────────────────────────
        static func severity(_ level: String) -> Color {
            switch level.lowercased() {
            case "info": return severityInfo
            case "low": return severityLow
            case "medium": return severityMedium
            case "high": return severityHigh
            case "critical": return severityCritical
            default: return textSecondary
            }
        }

        static func safety(_ rating: String) -> Color {
            switch rating.lowercased() {
            case "safe": return statusSafe
            case "review": return statusReview
            case "protected", "danger": return statusDanger
            default: return textTertiary
            }
        }
    }

    // MARK: - Spacing Tokens

    enum spacing {
        static let xxs: CGFloat = 2
        static let xs: CGFloat = 4
        static let sm: CGFloat = 8
        static let md: CGFloat = 12
        static let lg: CGFloat = 16
        static let xl: CGFloat = 20
        static let xxl: CGFloat = 24
        static let xxxl: CGFloat = 32

        // Semantic
        static let cardPadding: CGFloat = lg
        static let cardSpacing: CGFloat = md
        static let sectionSpacing: CGFloat = xl
        static let componentSpacing: CGFloat = sm
        static let listItemSpacing: CGFloat = sm
        static let sidebarItemIndent: CGFloat = lg
    }

    // MARK: - Corner Radius Tokens

    enum radius {
        static let xs: CGFloat = 4
        static let sm: CGFloat = 6
        static let md: CGFloat = 8
        static let lg: CGFloat = 10
        static let xl: CGFloat = 12
        static let xxl: CGFloat = 16

        // Semantic
        static let card: CGFloat = xl
        static let button: CGFloat = md
        static let badge: CGFloat = xs
        static let pill = CGFloat.infinity // capsule
    }

    // MARK: - Border Tokens

    enum border {
        static let thin: CGFloat = 0.5
        static let standard: CGFloat = 1
        static let thick: CGFloat = 2
    }

    // MARK: - Shadow Tokens

    enum shadow {
        static let card = ShadowSpec(opacity: 0.04, radius: 6, x: 0, y: 0)
        static let glow = ShadowSpec(opacity: 0.35, radius: 8, x: 0, y: 0)
        static let heroGlow = ShadowSpec(opacity: 0.40, radius: 14, x: 0, y: 2)
        static let activeNav = ShadowSpec(opacity: 0.20, radius: 3, x: 0, y: 0)
    }

    struct ShadowSpec {
        let opacity: Double
        let radius: CGFloat
        let x: CGFloat
        let y: CGFloat
    }

    // MARK: - Typography Tokens

    enum typography {
        // Hero
        static let heroLarge = Font.system(size: 48, weight: .bold, design: .rounded)
        static let heroTitle = Font.system(size: 32, weight: .bold, design: .rounded)
        static let heroSubtitle = Font.system(size: 24, weight: .bold, design: .rounded)

        // Headers
        static let sectionHeader = Font.system(size: 14, weight: .semibold)
        static let cardTitle = Font.system(size: 15, weight: .semibold)
        static let navItem = Font.system(size: 13, weight: .regular)
        static let navItemActive = Font.system(size: 13, weight: .semibold)

        // Body
        static let body = Font.system(size: 13, weight: .regular)
        static let bodyMedium = Font.system(size: 13, weight: .medium)
        static let bodySemibold = Font.system(size: 13, weight: .semibold)
        static let caption = Font.system(size: 12, weight: .regular)
        static let captionMedium = Font.system(size: 12, weight: .medium)

        // Small
        static let micro = Font.system(size: 11, weight: .regular)
        static let microMedium = Font.system(size: 11, weight: .medium)
        static let label = Font.system(size: 10, weight: .regular)

        // Monospaced
        static let mono = Font.system(size: 11, weight: .regular, design: .monospaced)
        static let monoSmall = Font.system(size: 10, weight: .regular, design: .monospaced)
        static let monoMicro = Font.system(size: 9, weight: .regular, design: .monospaced)

        // Logo
        static let logo = Font.system(size: 18, weight: .bold, design: .rounded)
    }

    // MARK: - Animation Tokens

    enum animation {
        static let fast = 0.15
        static let standard = 0.25
        static let slow = 0.35
        static let chartFill = 0.8

        static var standardSpring: SwiftUI.Animation {
            .spring(response: 0.5, dampingFraction: 0.85)
        }

        static var gentleSpring: SwiftUI.Animation {
            .spring(response: 0.4, dampingFraction: 0.9)
        }

        /// Returns the appropriate animation, respecting reduced motion.
        static func resolved(reduceMotion: Bool, duration: Double = standard) -> Animation? {
            reduceMotion ? nil : .easeInOut(duration: duration)
        }
    }

    // MARK: - Gradients

    enum gradient {
        static var neural: LinearGradient {
            LinearGradient(
                colors: [color.accent, color.teal],
                startPoint: .leading,
                endPoint: .trailing
            )
        }

        static var hud: LinearGradient {
            LinearGradient(
                colors: [color.accentBright, Color.adaptive(
                    dark: Color(red: 0.10, green: 0.40, blue: 0.80),
                    light: Color(red: 0.10, green: 0.35, blue: 0.70)
                )],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
        }

        static var metallic: LinearGradient {
            LinearGradient(
                colors: [color.metallic, color.metallicWarm],
                startPoint: .top,
                endPoint: .bottom
            )
        }

        static var voidBackground: LinearGradient {
            LinearGradient(
                colors: [color.backgroundCanvas, color.backgroundPrimary],
                startPoint: .top,
                endPoint: .bottom
            )
        }

        static var card: LinearGradient {
            LinearGradient(
                colors: [color.cardBackground, color.backgroundSecondary],
                startPoint: .top,
                endPoint: .bottom
            )
        }

        static var sidebar: LinearGradient {
            LinearGradient(
                colors: [color.backgroundSecondary, color.backgroundCanvas],
                startPoint: .top,
                endPoint: .bottom
            )
        }
    }

    // MARK: - Legacy Aliases (backward compatibility)
    // These maintain the existing API surface during migration.
    // New code should use the semantic namespaces above.

    static let bgVoid = color.backgroundCanvas
    static let bgPrimary = color.backgroundPrimary
    static let bgSecondary = color.backgroundSecondary
    static let bgTertiary = color.backgroundTertiary
    static let accent = color.accent
    static let accentBright = color.accentBright
    static let accentDim = color.accentDim
    static let neuralBlue = color.neuralBlue
    static let metallic = color.metallic
    static let metallicWarm = color.metallicWarm
    static let steel = color.steel
    static let teal = color.teal
    static let tealDeep = color.tealDeep
    static let info = color.severityInfo
    static let low = color.severityLow
    static let medium = color.severityMedium
    static let high = color.severityHigh
    static let critical = color.severityCritical
    static let safe = color.statusSafe
    static let warning = color.statusWarning
    static let danger = color.statusDanger
    static let anomaly = color.statusAnomaly
    static let textPrimary = color.textPrimary
    static let textSecondary = color.textSecondary
    static let textTertiary = color.textTertiary
    static let cardBg = color.cardBackground
    static let cardBorder = color.cardBorder

    static var neuralGradient: LinearGradient { gradient.neural }
    static var hudGradient: LinearGradient { gradient.hud }
    static var metallicGradient: LinearGradient { gradient.metallic }
    static var voidGradient: LinearGradient { gradient.voidBackground }
    static var cardGradient: LinearGradient { gradient.card }
    static var sidebarGradient: LinearGradient { gradient.sidebar }
}

// MARK: - Adaptive Color

extension Color {
    /// Creates a color that adapts between dark and light appearances.
    static func adaptive(dark: Color, light: Color) -> Color {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .vibrantDark]) != nil
                ? NSColor(dark)
                : NSColor(light)
        }.toColor()
    }
}

extension NSColor {
    func toColor() -> Color {
        Color(self)
    }
}

// MARK: - XCard (Design System Component)

struct XCard<Content: View>: View {
    let content: Content
    var glow: Bool = true
    var padding: CGFloat = XTheme.spacing.cardPadding

    init(glow: Bool = true, padding: CGFloat = XTheme.spacing.cardPadding, @ViewBuilder content: () -> Content) {
        self.glow = glow
        self.padding = padding
        self.content = content()
    }

    var body: some View {
        content
            .padding(padding)
            .background(XTheme.gradient.card)
            .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.card))
            .overlay(
                RoundedRectangle(cornerRadius: XTheme.radius.card)
                    .stroke(XTheme.color.cardBorder, lineWidth: XTheme.border.standard)
            )
            .shadow(
                color: XTheme.color.accent.opacity(glow ? XTheme.shadow.card.opacity : 0),
                radius: XTheme.shadow.card.radius
            )
            .accessibilityElement(children: .contain)
    }
}

// MARK: - XSectionHeader (Design System Component)

struct XSectionHeader: View {
    let title: String
    let icon: String
    var count: Int? = nil

    var body: some View {
        HStack(spacing: XTheme.spacing.sm) {
            Image(systemName: icon)
                .foregroundStyle(XTheme.color.accent)
                .font(XTheme.typography.sectionHeader)
                .xGlow(XTheme.color.accent, radius: 4)
                .accessibilityLabel("\(title) section")
            Text(title)
                .font(XTheme.typography.sectionHeader)
                .foregroundStyle(XTheme.color.textPrimary)
            if let c = count {
                Text("\(c)")
                    .font(XTheme.typography.microMedium)
                    .foregroundStyle(XTheme.color.accent)
                    .padding(.horizontal, XTheme.spacing.xs + 2)
                    .padding(.vertical, XTheme.spacing.xxs)
                    .background(XTheme.color.accent.opacity(0.12))
                    .clipShape(Capsule())
                    .accessibilityLabel("\(c) items")
            }
            Spacer()
        }
        .accessibilityElement(children: .combine)
    }
}

// MARK: - Glow Extensions

extension View {
    /// Applies a neural-network glow shadow to the view.
    /// Use sparingly — glow is a semantic signal, not decoration.
    func xGlow(_ color: Color = XTheme.color.accent, radius: CGFloat = XTheme.shadow.glow.radius) -> some View {
        self.shadow(color: color.opacity(XTheme.shadow.glow.opacity), radius: radius)
    }

    /// Stronger glow for hero elements.
    func xHeroGlow(_ color: Color = XTheme.color.accent, radius: CGFloat = XTheme.shadow.heroGlow.radius) -> some View {
        self.shadow(
            color: color.opacity(XTheme.shadow.heroGlow.opacity),
            radius: radius,
            x: XTheme.shadow.heroGlow.x,
            y: XTheme.shadow.heroGlow.y
        )
    }

    /// Metallic text style — applies the metallic gradient as foreground.
    func xMetallicText() -> some View {
        self.foregroundStyle(XTheme.gradient.metallic)
    }
}

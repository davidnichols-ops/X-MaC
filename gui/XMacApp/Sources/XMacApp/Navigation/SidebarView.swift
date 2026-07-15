import SwiftUI

// MARK: - SidebarView
//
// Clean 5-section sidebar with consistent nesting.
// Replaces the 10-tab sidebar with mixed nesting strategies.

struct SidebarView: View {
    @EnvironmentObject var router: AppRouter
    @EnvironmentObject var runner: XMacRunner
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        VStack(spacing: 0) {
            // Logo
            logoSection
                .padding(.vertical, XTheme.spacing.lg)
                .padding(.horizontal, XTheme.spacing.md)

            Divider()
                .overlay(XTheme.color.cardBorder)

            // Navigation
            ScrollView {
                VStack(alignment: .leading, spacing: XTheme.spacing.md) {
                    // Home (standalone, no section header)
                    navButton(
                        destination: .home,
                        icon: NavigationDestination.home.icon,
                        label: NavigationDestination.home.label,
                        indent: 0
                    )

                    // Sectioned navigation
                    ForEach(NavigationSection.allCases.filter { $0 != .home }) { section in
                        sidebarSection(section)
                    }
                }
                .padding(.horizontal, XTheme.spacing.sm)
                .padding(.vertical, XTheme.spacing.md)
            }

            Spacer()

            // Bottom status
            bottomStatus
                .padding(.horizontal, XTheme.spacing.md)
                .padding(.vertical, XTheme.spacing.sm)
        }
        .background(XTheme.gradient.sidebar)
        .frame(minWidth: 220, idealWidth: 240)
    }

    // MARK: - Logo

    private var logoSection: some View {
        HStack(spacing: XTheme.spacing.sm) {
            Image(systemName: "cpu")
                .font(.system(size: 24, weight: .bold))
                .foregroundStyle(XTheme.gradient.metallic)
                .xGlow(XTheme.color.accent, radius: 6)
                .accessibilityLabel("X-MaC logo")
            Text("X-MaC")
                .font(XTheme.typography.logo)
                .foregroundStyle(XTheme.gradient.metallic)
                .accessibilityAddTraits(.isHeader)
        }
    }

    // MARK: - Section

    @ViewBuilder
    private func sidebarSection(_ section: NavigationSection) -> some View {
        VStack(alignment: .leading, spacing: XTheme.spacing.xxs) {
            // Section header
            Text(section.label.uppercased())
                .font(XTheme.typography.microMedium)
                .foregroundStyle(XTheme.color.textTertiary)
                .padding(.horizontal, XTheme.spacing.md)
                .padding(.top, XTheme.spacing.sm)
                .accessibilityAddTraits(.isHeader)

            // Destinations
            ForEach(section.destinations) { destination in
                navButton(
                    destination: destination,
                    icon: destination.icon,
                    label: destination.label,
                    indent: 0
                )

                // Sub-destinations (nested)
                if router.selectedDestination == destination,
                   let subs = destination.subDestinations {
                    ForEach(subs) { sub in
                        subButton(destination: destination, sub: sub)
                    }
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
        }
    }

    // MARK: - Nav Button

    private func navButton(
        destination: NavigationDestination,
        icon: String,
        label: String,
        indent: Int
    ) -> some View {
        let isActive = router.selectedDestination == destination

        return Button {
            withAnimation(XTheme.animation.resolved(reduceMotion: reduceMotion) ?? .easeInOut) {
                router.navigate(to: destination)
            }
        } label: {
            HStack(spacing: XTheme.spacing.sm) {
                Image(systemName: icon)
                    .font(.system(size: 14, weight: .medium))
                    .frame(width: 20)
                    .foregroundStyle(isActive ? XTheme.color.accent : XTheme.color.textSecondary)
                    .if(isActive) { $0.xGlow(XTheme.color.accent, radius: 3) }

                Text(label)
                    .font(isActive ? XTheme.typography.navItemActive : XTheme.typography.navItem)
                    .foregroundStyle(isActive ? XTheme.color.textPrimary : XTheme.color.textSecondary)

                Spacer()
            }
            .padding(.horizontal, XTheme.spacing.md)
            .padding(.leading, CGFloat(indent) * XTheme.spacing.sidebarItemIndent)
            .padding(.vertical, XTheme.spacing.sm + 2)
            .background(
                Group {
                    if isActive {
                        LinearGradient(
                            colors: [XTheme.color.accent.opacity(0.15), XTheme.color.accent.opacity(0.05)],
                            startPoint: .leading,
                            endPoint: .trailing
                        )
                    } else {
                        Color.clear
                    }
                }
            )
            .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.md))
            .overlay(
                RoundedRectangle(cornerRadius: XTheme.radius.md)
                    .stroke(XTheme.color.accent.opacity(isActive ? 0.2 : 0), lineWidth: XTheme.border.standard)
            )
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
        .accessibilityValue(isActive ? "selected" : "not selected")
        .accessibilityAddTraits(isActive ? .isSelected : [])
    }

    // MARK: - Sub Button

    private func subButton(destination: NavigationDestination, sub: SubDestination) -> some View {
        let isActive = router.selectedDestination == destination
            && router.selectedSubDestination == sub

        return Button {
            withAnimation(XTheme.animation.resolved(reduceMotion: reduceMotion) ?? .easeInOut) {
                router.navigate(to: destination)
                router.navigate(toSub: sub)
            }
        } label: {
            HStack(spacing: XTheme.spacing.sm) {
                Image(systemName: sub.icon)
                    .font(.system(size: 12, weight: .medium))
                    .frame(width: 18)
                    .foregroundStyle(isActive ? XTheme.color.accent : XTheme.color.textTertiary)

                Text(sub.label)
                    .font(.system(size: 12, weight: isActive ? .semibold : .regular))
                    .foregroundStyle(isActive ? XTheme.color.textPrimary : XTheme.color.textSecondary)

                Spacer()
            }
            .padding(.horizontal, XTheme.spacing.md)
            .padding(.leading, XTheme.spacing.sidebarItemIndent + XTheme.spacing.md)
            .padding(.vertical, XTheme.spacing.sm)
            .background(
                isActive
                    ? XTheme.color.accent.opacity(0.1)
                    : Color.clear
            )
            .clipShape(RoundedRectangle(cornerRadius: XTheme.radius.sm))
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(sub.label)")
        .accessibilityValue(isActive ? "selected" : "not selected")
        .accessibilityAddTraits(isActive ? .isSelected : [])
    }

    // MARK: - Bottom Status

    private var bottomStatus: some View {
        HStack(spacing: XTheme.spacing.xs) {
            Circle()
                .fill(runner.isScanning ? XTheme.color.accent : XTheme.color.statusSafe)
                .frame(width: 8, height: 8)
                .if(runner.isScanning) { $0.xGlow(XTheme.color.accent, radius: 4) }
                .accessibilityLabel(runner.isScanning ? "Scanning" : "Idle")

            Text(runner.isScanning ? runner.scanPhase : "Ready")
                .font(XTheme.typography.micro)
                .foregroundStyle(XTheme.color.textTertiary)
                .lineLimit(1)
        }
    }
}

// MARK: - Conditional Modifier

extension View {
    @ViewBuilder
    func `if`<Transform: View>(_ condition: Bool, transform: (Self) -> Transform) -> some View {
        if condition {
            transform(self)
        } else {
            self
        }
    }
}

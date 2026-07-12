import SwiftUI

struct OnboardingView: View {
    @AppStorage("hasCompletedOnboarding") private var hasCompletedOnboarding: Bool = false

    let onComplete: () -> Void
    let onQuickScan: () -> Void

    @State private var pageIndex = 0
    @State private var particles = (0..<6).map { _ in OnboardingParticle() }
    @State private var particleOffsets = Array(repeating: CGFloat(0), count: 6)

    private let pages: [OnboardingPage] = [
        OnboardingPage(
            icon: "shield.checkered",
            iconColors: [XTheme.accent, XTheme.accentDim],
            title: "Meet X-MaC",
            body: "The only Mac cleaner that explains every decision before touching a single file."
        ),
        OnboardingPage(
            icon: "sparkles",
            iconColors: [XTheme.low, Color(red: 0.20, green: 0.55, blue: 0.30)],
            title: "One-tap clean, zero surprises",
            body: "Run a Smart Scan in seconds. X-MaC scores every file with an on-device neural network — no cloud, no guessing."
        ),
        OnboardingPage(
            icon: "brain",
            iconColors: [XTheme.anomaly, Color(red: 0.45, green: 0.25, blue: 0.70)],
            title: "GNN scoring, on your device",
            body: "A Graph Neural Network maps relationships between files and directories, catching bloat that simple rules miss."
        ),
        OnboardingPage(
            icon: "hand.raised.fill",
            iconColors: [XTheme.medium, XTheme.high],
            title: "Nothing moves without you",
            body: "Every cleanup goes to Trash first. Protected system paths are blocked. You review, you decide, you act."
        ),
        OnboardingPage(
            icon: "checkmark.circle.fill",
            iconColors: [XTheme.safe, Color(red: 0.20, green: 0.60, blue: 0.30)],
            title: "You're all set",
            body: "Click Quick Scan below to find space you can safely reclaim right now."
        )
    ]

    var body: some View {
        ZStack {
            backdrop
            radialGlow
            particlesLayer
            contentLayer
        }
    }

    // MARK: - Background

    private var backdrop: some View {
        XTheme.voidGradient
            .ignoresSafeArea()
    }

    private var radialGlow: some View {
        GeometryReader { geo in
            RadialGradient(
                gradient: Gradient(colors: [
                    Color(red: 0.10, green: 0.20, blue: 0.40).opacity(0.35),
                    Color.clear
                ]),
                center: .center,
                startRadius: 0,
                endRadius: geo.size.width * 0.6
            )
            .ignoresSafeArea()
        }
    }

    // MARK: - Particles

    private var particlesLayer: some View {
        GeometryReader { geo in
            ForEach(particles.indices, id: \.self) { i in
                let particle = particles[i]
                Circle()
                    .fill(Color.white.opacity(0.15))
                    .frame(width: particle.size, height: particle.size)
                    .position(
                        x: particle.x * geo.size.width,
                        y: particle.y * geo.size.height
                    )
                    .offset(y: particleOffsets[i])
                    .blur(radius: 0.5)
            }
        }
        .ignoresSafeArea()
        .onAppear(perform: startParticleAnimation)
    }

    private func startParticleAnimation() {
        for i in particleOffsets.indices {
            particleOffsets[i] = 0
            withAnimation(
                .easeInOut(duration: particles[i].duration)
                .repeatForever(autoreverses: true)
            ) {
                particleOffsets[i] = -50
            }
        }
    }

    // MARK: - Content

    private var contentLayer: some View {
        VStack(spacing: 0) {
            skipButton

            Spacer()

            pageContent
                .padding(.horizontal, 40)

            Spacer()

            bottomControls
                .padding(.bottom, 40)
        }
    }

    private var skipButton: some View {
        HStack {
            Spacer()
            Button("Skip") { finish() }
                .buttonStyle(.plain)
                .font(.system(size: 14, weight: .medium))
                .foregroundStyle(Color(white: 0.65))
                .padding(.top, 24)
                .padding(.trailing, 32)
                .contentShape(Rectangle())
        }
    }

    @ViewBuilder
    private var pageContent: some View {
        let page = pages[pageIndex]
        let isWelcome = pageIndex == 0

        VStack(spacing: 24) {
            Image(systemName: page.icon)
                .font(.system(size: isWelcome ? 72 : 56, weight: .semibold))
                .foregroundStyle(
                    LinearGradient(
                        colors: page.iconColors,
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .xGlow(page.iconColors[0])

            Text(page.title)
                .font(.system(size: isWelcome ? 34 : 28, weight: .bold))
                .foregroundStyle(XTheme.metallicGradient)
                .multilineTextAlignment(.center)

            Text(page.body)
                .font(.system(size: 14))
                .foregroundStyle(Color(white: 0.65))
                .multilineTextAlignment(.center)
                .frame(maxWidth: 400)
                .fixedSize(horizontal: false, vertical: true)
        }
        .id(pageIndex)
        .transition(
            .asymmetric(
                insertion: .move(edge: .trailing).combined(with: .opacity),
                removal: .move(edge: .leading).combined(with: .opacity)
            )
        )
        .animation(.spring(response: 0.5, dampingFraction: 0.85), value: pageIndex)
    }

    private var bottomControls: some View {
        VStack(spacing: 32) {
            pageDots

            if pageIndex == pages.count - 1 {
                quickScanButton
            } else {
                nextButton
            }

            Button("Already used X-MaC?") { finish() }
                .buttonStyle(.plain)
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Color(white: 0.55))
        }
    }

    private var pageDots: some View {
        HStack(spacing: 12) {
            ForEach(0..<pages.count, id: \.self) { index in
                Circle()
                    .fill(index == pageIndex ? XTheme.accent : Color(white: 0.35))
                    .frame(width: 8, height: 8)
                    .scaleEffect(index == pageIndex ? 1.25 : 1.0)
                    .animation(.easeInOut(duration: 0.25), value: pageIndex)
            }
        }
    }

    private var nextButton: some View {
        Button(action: nextPage) {
            HStack(spacing: 8) {
                Text("Next")
                    .font(.system(size: 16, weight: .semibold))
                Image(systemName: "arrow.right")
                    .font(.system(size: 14, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 24)
            .frame(height: 44)
            .background(XTheme.neuralGradient)
            .clipShape(Capsule())
        }
        .buttonStyle(.plain)
        .keyboardShortcut(.return, modifiers: [])
    }

    private var quickScanButton: some View {
        Button(action: runQuickScan) {
            HStack(spacing: 8) {
                Text("Run Quick Scan")
                    .font(.system(size: 16, weight: .semibold))
                Image(systemName: "arrow.right")
                    .font(.system(size: 14, weight: .semibold))
            }
            .foregroundStyle(.white)
            .padding(.horizontal, 28)
            .frame(height: 48)
            .background(XTheme.neuralGradient)
            .clipShape(Capsule())
            .xGlow(XTheme.accent, radius: 16)
        }
        .buttonStyle(.plain)
        .keyboardShortcut(.return, modifiers: [])
    }

    // MARK: - Actions

    private func nextPage() {
        pageIndex = min(pageIndex + 1, pages.count - 1)
    }

    private func finish() {
        hasCompletedOnboarding = true
        onComplete()
    }

    private func runQuickScan() {
        hasCompletedOnboarding = true
        onQuickScan()
    }
}

// MARK: - Models

private struct OnboardingPage {
    let icon: String
    let iconColors: [Color]
    let title: String
    let body: String
}

private struct OnboardingParticle {
    let x: CGFloat
    let y: CGFloat
    let size: CGFloat
    let duration: Double

    init() {
        self.x = CGFloat.random(in: 0.1...0.9)
        self.y = CGFloat.random(in: 0.2...0.9)
        self.size = CGFloat.random(in: 3...5)
        self.duration = Double.random(in: 3.5...4.5)
    }
}

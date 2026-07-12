import SwiftUI

// MARK: - Volume Stats (parsed from Rust hint JSON)

struct VolumeStats {
    let total: Int
    let used: Int       // system + data combined
    let free: Int
    let system: Int     // macOS sealed system volume
    let data: Int       // writable data volume (home + apps + misc)
    let apps: Int       // /Applications physical size (subset of data)

    /// Everything in the data volume that isn't /Applications.
    var otherData: Int { max(0, data - apps) }

    var usedFraction: Double { total > 0 ? Double(used) / Double(total) : 0 }
    var freeFraction: Double { total > 0 ? Double(free) / Double(total) : 0 }
    var usedPercent: String { String(format: "%.0f%%", usedFraction * 100) }
}

// MARK: - Disk Category for colour-coded segments

struct DiskSegment: Identifiable {
    let id = UUID()
    let label: String
    let subtitle: String    // shown in donut centre on hover
    let bytes: Int
    let color: Color
    let icon: String
    var fraction: Double = 0
}

// MARK: - Main View

struct DiskView: View {
    @EnvironmentObject var runner: XMacRunner

    // Items whose title starts "Disk usage:" (directories)
    private var dirFindings: [Finding] {
        runner.findings.filter { $0.category == "system_info" && $0.title.hasPrefix("Disk usage:") }
    }

    // Items whose title starts "Large file:" (individual files)
    private var fileFindings: [Finding] {
        runner.findings.filter { $0.category == "large_file" || $0.title.hasPrefix("Large file:") }
    }

    // The volume summary finding (title starts "Volume:")
    private var volumeFinding: Finding? {
        runner.findings.first { $0.title.hasPrefix("Volume:") }
    }

    private var volumeStats: VolumeStats? {
        guard let f = volumeFinding,
              let hint = f.remediation_hint,
              let data = hint.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        else { return nil }

        func int(_ key: String) -> Int {
            (obj[key] as? Int) ?? (obj[key] as? Double).map(Int.init) ?? 0
        }
        let total  = int("volume_total")
        let used   = int("volume_used")
        let free   = int("volume_free")
        let system = int("volume_system")
        let dataV  = int("volume_data")
        let apps   = int("volume_apps")
        guard total > 0 else { return nil }
        return VolumeStats(total: total, used: used, free: free,
                           system: system, data: dataV, apps: apps)
    }

    private var allItemFindings: [Finding] {
        (dirFindings + fileFindings).sorted { ($0.size_bytes ?? 0) > ($1.size_bytes ?? 0) }
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                if runner.isScanning {
                    ScanProgressView(phase: runner.scanPhase)
                } else if let err = runner.error {
                    ScanErrorView(message: err)
                } else if dirFindings.isEmpty && fileFindings.isEmpty {
                    DiskEmptyState()
                } else {
                    // Donut + legend header
                    if let stats = volumeStats {
                        DiskDonutCard(stats: stats, segments: buildSegments(stats: stats))
                    }

                    // Quick clean bar
                    if !allItemFindings.filter({ isSafeForDisk($0.target.value) }).isEmpty {
                        DiskQuickCleanBar(findings: allItemFindings)
                    }

                    // Directory breakdown
                    if !dirFindings.isEmpty {
                        DiskBreakdownList(
                            title: "Directories",
                            icon: "folder.fill",
                            findings: dirFindings,
                            total: volumeStats?.total ?? 1
                        )
                    }

                    // Large files
                    if !fileFindings.isEmpty {
                        DiskBreakdownList(
                            title: "Large Files",
                            icon: "doc.fill",
                            findings: fileFindings,
                            total: volumeStats?.total ?? 1
                        )
                    }
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }

    // Build properly-labelled donut segments:
    //   macOS System | Applications | top home dirs (named) | Other Home | Free
    private func buildSegments(stats: VolumeStats) -> [DiskSegment] {
        // Palette distilled from the X-MaC icon: cyan → teal → neural blue → purple
        let palette: [Color] = [
            XTheme.accent,                                 // cyan  – Library
            XTheme.teal,                                   // teal
            XTheme.safe,                                   // neon green
            XTheme.warning,                                // amber
            XTheme.high,                                   // orange
            XTheme.accentBright,                           // bright cyan
            XTheme.anomaly,                                // neural purple
        ]

        var segments: [DiskSegment] = []

        func frac(_ bytes: Int) -> Double {
            stats.total > 0 ? Double(bytes) / Double(stats.total) : 0
        }

        // 1. macOS System volume (sealed, user can't clean this)
        if stats.system > 0 {
            segments.append(DiskSegment(
                label: "macOS System",
                subtitle: "Sealed read-only OS volume — cannot be cleaned",
                bytes: stats.system,
                color: XTheme.steel,  // metallic grey — matches the X in the icon
                icon: "apple.logo",
                fraction: frac(stats.system)
            ))
        }

        // 2. /Applications (named separately so users know its weight)
        if stats.apps > 0 {
            segments.append(DiskSegment(
                label: "Applications",
                subtitle: "/Applications + /System/Applications",
                bytes: stats.apps,
                color: XTheme.teal,  // teal — matches icon's mid-tones
                icon: "square.stack.3d.up.fill",
                fraction: frac(stats.apps)
            ))
        }

        // 3. Top home directories — capped at 6 for legibility
        let sortedDirs = dirFindings.sorted { ($0.size_bytes ?? 0) > ($1.size_bytes ?? 0) }
        let topDirs = sortedDirs.prefix(6)

        let dirSubtitles: [String: String] = [
            "Library":    "App support, caches, logs, mail",
            "Desktop":    "Files saved to your Desktop",
            "Downloads":  "Downloaded files",
            "Documents":  "Documents folder",
            "Movies":     "Video files",
            "Music":      "Music library",
            "Pictures":   "Photos and images",
            "Projects":   "Development projects",
            ".cache":     "App cache data (hidden)",
            ".local":     "Local user data (hidden)",
            ".rustup":    "Rust toolchain (hidden)",
            ".cargo":     "Cargo registry & bins (hidden)",
            ".pyenv":     "Python environments (hidden)",
            ".nvm":       "Node version manager (hidden)",
            ".mlxstudio": "MLX Studio models (hidden)",
            ".lmstudio":  "LM Studio models (hidden)",
        ]

        for (i, f) in topDirs.enumerated() {
            let bytes = f.size_bytes ?? 0
            guard bytes > 0 else { continue }
            let name = (f.target.value as NSString).lastPathComponent
            let sub = dirSubtitles[name] ?? "Home directory: ~/\(name)"
            segments.append(DiskSegment(
                label: name,
                subtitle: sub,
                bytes: bytes,
                color: palette[i % palette.count],
                icon: "folder.fill",
                fraction: frac(bytes)
            ))
        }

        // Sum ALL scanned dirs to minimise "Other Home"
        let allHomeDirs = dirFindings.reduce(0) { $0 + ($1.size_bytes ?? 0) }

        // 4. Other Home Data = Data volume minus /Applications minus all named dirs
        let otherHome = max(0, stats.otherData - allHomeDirs)
        if otherHome > 50_000_000 {
            segments.append(DiskSegment(
                label: "Other Home Data",
                subtitle: "Hidden files, dot-folders, and unlisted directories",
                bytes: otherHome,
                color: Color(white: 0.45),
                icon: "ellipsis.circle.fill",
                fraction: frac(otherHome)
            ))
        }

        // 5. Free — always last, dark gray
        segments.append(DiskSegment(
            label: "Free Space",
            subtitle: "Available space on Macintosh HD",
            bytes: stats.free,
            color: Color(white: 0.20),
            icon: "circle.dashed",
            fraction: frac(stats.free)
        ))

        return segments
    }
}

// MARK: - Empty State

private struct DiskEmptyState: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "internaldrive")
                .font(.system(size: 52, weight: .thin))
                .foregroundStyle(XTheme.textTertiary)
            Text("No disk analysis yet")
                .font(.system(size: 18, weight: .semibold))
                .foregroundStyle(XTheme.textPrimary)
            Text("Run a Disk Scan to map your storage usage across every folder and large file.")
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textSecondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 380)
            Button {
                runner.startDiskScan()
            } label: {
                Label("Scan Disk Now", systemImage: "internaldrive")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 22)
                    .padding(.vertical, 11)
                    .background(XTheme.neuralGradient)
                    .clipShape(Capsule())
            }
            .buttonStyle(.plain)
            .xGlow()
        }
        .frame(maxWidth: .infinity)
        .padding(40)
    }
}

// MARK: - Donut Card

struct DiskDonutCard: View {
    let stats: VolumeStats
    let segments: [DiskSegment]

    @State private var hovered: DiskSegment? = nil
    @State private var animateFill = false

    var body: some View {
        XCard {
            HStack(alignment: .top, spacing: 32) {
                // Donut wheel — mouse hit-test via onContinuousHover
                ZStack {
                    DiskDonutShape(segments: segments, hovered: hovered, animate: animateFill)
                        .frame(width: 220, height: 220)
                        .onContinuousHover { phase in
                            switch phase {
                            case .active(let loc):
                                hovered = hitSegment(at: loc, size: CGSize(width: 220, height: 220))
                            case .ended:
                                withAnimation(.easeInOut(duration: 0.12)) { hovered = nil }
                            }
                        }

                    // Centre label — updates on hover
                    VStack(spacing: 3) {
                        if let h = hovered {
                            Image(systemName: h.icon)
                                .font(.system(size: 16, weight: .semibold))
                                .foregroundStyle(h.color)
                            Text(formatBytes(h.bytes))
                                .font(.system(size: 17, weight: .bold))
                                .foregroundStyle(XTheme.metallicGradient)
                                .xGlow(XTheme.accent, radius: 4)
                            Text(h.label)
                                .font(.system(size: 11, weight: .semibold))
                                .foregroundStyle(h.color)
                            Text(String(format: "%.1f%%", h.fraction * 100))
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textTertiary)
                            Text(h.subtitle)
                                .font(.system(size: 9))
                                .foregroundStyle(XTheme.textTertiary)
                                .multilineTextAlignment(.center)
                                .frame(maxWidth: 110)
                                .lineLimit(2)
                        } else {
                            Text(stats.usedPercent)
                                .font(.system(size: 26, weight: .bold))
                                .foregroundStyle(XTheme.metallicGradient)
                                .xGlow(XTheme.accent, radius: 4)
                            Text("used")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                            Text(formatBytes(stats.total))
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                    }
                    .animation(.easeInOut(duration: 0.15), value: hovered?.id)
                }

                // Legend
                VStack(alignment: .leading, spacing: 0) {
                    Text("Storage Map")
                        .font(.system(size: 15, weight: .bold))
                        .foregroundStyle(XTheme.textPrimary)
                        .padding(.bottom, 14)

                    // Volume summary row
                    HStack(spacing: 16) {
                        VolumeStatPill(label: "Total", value: formatBytes(stats.total), color: XTheme.textTertiary)
                        VolumeStatPill(label: "Used", value: formatBytes(stats.used), color: XTheme.accent)
                        VolumeStatPill(label: "Free", value: formatBytes(stats.free), color: XTheme.safe)
                    }
                    .padding(.bottom, 16)

                    Divider()
                        .background(XTheme.cardBorder)
                        .padding(.bottom, 12)

                    // Segment legend rows (exclude Free Space — shown in donut only)
                    ForEach(segments.filter { $0.label != "Free Space" }) { seg in
                        DiskLegendRow(segment: seg, total: stats.total, isHovered: hovered?.id == seg.id)
                            .onHover { inside in
                                withAnimation(.easeInOut(duration: 0.12)) {
                                    hovered = inside ? seg : nil
                                }
                            }
                            .padding(.bottom, 6)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(4)
        }
        .onAppear {
            withAnimation(.easeOut(duration: 0.8)) { animateFill = true }
        }
    }

    /// Given a mouse location inside the donut frame, returns the segment the
    /// cursor is over, or nil if it's in the centre hole or outside the ring.
    private func hitSegment(at point: CGPoint, size: CGSize) -> DiskSegment? {
        let centre = CGPoint(x: size.width / 2, y: size.height / 2)
        let dx = point.x - centre.x
        let dy = point.y - centre.y
        let dist = sqrt(dx * dx + dy * dy)

        let outerR: CGFloat = min(size.width, size.height) / 2 - 2
        let innerR: CGFloat = outerR - 34   // matches DiskDonutShape thickness

        guard dist >= innerR && dist <= outerR + 6 else { return nil }

        // Angle from 12-o'clock, clockwise
        var angle = atan2(dy, dx) + .pi / 2
        if angle < 0 { angle += 2 * .pi }

        let gap: CGFloat = 0.012
        var start: CGFloat = 0
        for seg in segments {
            let sweep = CGFloat(seg.fraction) * 2 * .pi - gap * 2
            guard sweep > 0 else { continue }
            let segStart = start + gap
            let segEnd   = segStart + sweep
            if angle >= segStart && angle <= segEnd {
                return seg
            }
            start += sweep + gap * 2
        }
        return nil
    }
}

// MARK: - Donut Shape (custom canvas)

struct DiskDonutShape: View {
    let segments: [DiskSegment]
    let hovered: DiskSegment?
    let animate: Bool

    private let thickness: CGFloat = 34
    private let gap: CGFloat = 0.012   // radians between segments

    var body: some View {
        Canvas { ctx, size in
            let centre = CGPoint(x: size.width / 2, y: size.height / 2)
            let outerR = min(size.width, size.height) / 2 - 2
            let innerR = outerR - thickness

            var startAngle = -CGFloat.pi / 2  // 12-o'clock

            for seg in segments {
                let fraction = animate ? seg.fraction : 0
                let sweep = CGFloat(fraction) * 2 * .pi - gap * 2
                guard sweep > 0 else { continue }

                let isHov = hovered?.id == seg.id
                let r = isHov ? outerR + 5 : outerR
                let iR = isHov ? innerR - 2 : innerR

                var path = Path()
                path.addArc(center: centre, radius: r,
                            startAngle: .radians(Double(startAngle + gap)),
                            endAngle: .radians(Double(startAngle + gap + sweep)),
                            clockwise: false)
                path.addArc(center: centre, radius: iR,
                            startAngle: .radians(Double(startAngle + gap + sweep)),
                            endAngle: .radians(Double(startAngle + gap)),
                            clockwise: true)
                path.closeSubpath()

                ctx.fill(path, with: .color(seg.color.opacity(isHov ? 1.0 : 0.85)))

                // Segment divider lines
                if segments.count > 1 {
                    var divider = Path()
                    let dx = cos(startAngle + gap) * r
                    let dy = sin(startAngle + gap) * r
                    let dx2 = cos(startAngle + gap) * iR
                    let dy2 = sin(startAngle + gap) * iR
                    divider.move(to: CGPoint(x: centre.x + dx2, y: centre.y + dy2))
                    divider.addLine(to: CGPoint(x: centre.x + dx, y: centre.y + dy))
                    ctx.stroke(divider, with: .color(XTheme.bgVoid.opacity(0.9)), lineWidth: 2)
                }

                startAngle += sweep + gap * 2
            }

            // Inner circle shadow ring
            let ringPath = Path(ellipseIn: CGRect(
                x: centre.x - innerR, y: centre.y - innerR,
                width: innerR * 2, height: innerR * 2
            ))
            ctx.fill(ringPath, with: .color(XTheme.bgSecondary))
        }
        .animation(.easeOut(duration: 0.9), value: animate)
    }
}

// MARK: - Legend Row

private struct DiskLegendRow: View {
    let segment: DiskSegment
    let total: Int
    let isHovered: Bool

    var body: some View {
        HStack(spacing: 10) {
            RoundedRectangle(cornerRadius: 3)
                .fill(segment.color)
                .frame(width: 10, height: 10)

            Image(systemName: segment.icon)
                .font(.system(size: 10))
                .foregroundStyle(segment.color)
                .frame(width: 14)

            Text(segment.label)
                .font(.system(size: 12, weight: isHovered ? .semibold : .regular))
                .foregroundStyle(isHovered ? XTheme.textPrimary : XTheme.textSecondary)
                .lineLimit(1)

            Spacer()

            Text(formatBytes(segment.bytes))
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(XTheme.textTertiary)

            // Fraction bar
            GeometryReader { geo in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 2)
                        .fill(XTheme.bgTertiary)
                        .frame(height: 4)
                    RoundedRectangle(cornerRadius: 2)
                        .fill(segment.color.opacity(0.85))
                        .frame(width: geo.size.width * CGFloat(segment.fraction), height: 4)
                }
            }
            .frame(width: 60, height: 4)
        }
        .padding(.vertical, 3)
        .padding(.horizontal, 6)
        .background(isHovered ? XTheme.bgTertiary.opacity(0.6) : Color.clear)
        .clipShape(RoundedRectangle(cornerRadius: 6))
        .animation(.easeInOut(duration: 0.12), value: isHovered)
    }
}

private struct VolumeStatPill: View {
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.system(size: 10))
                .foregroundStyle(XTheme.textTertiary)
            Text(value)
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(color)
        }
    }
}

// MARK: - Breakdown List (directories or files)

struct DiskBreakdownList: View {
    let title: String
    let icon: String
    let findings: [Finding]
    let total: Int

    @State private var expanded = true

    private var sorted: [Finding] {
        findings.sorted { ($0.size_bytes ?? 0) > ($1.size_bytes ?? 0) }
    }

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                Button {
                    withAnimation(.easeInOut(duration: 0.2)) { expanded.toggle() }
                } label: {
                    HStack {
                        XSectionHeader(title: title, icon: icon, count: findings.count)
                        Spacer()
                        Image(systemName: expanded ? "chevron.up" : "chevron.down")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                }
                .buttonStyle(.plain)

                if expanded {
                    LazyVStack(spacing: 0) {
                        ForEach(sorted) { finding in
                            DiskItemRow(finding: finding, total: total)
                            if finding.id != sorted.last?.id {
                                Divider()
                                    .background(XTheme.cardBorder.opacity(0.5))
                                    .padding(.leading, 36)
                            }
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Disk Item Row

struct DiskItemRow: View {
    let finding: Finding
    let total: Int

    @State private var showTrashConfirm = false
    @State private var isTrashed = false

    private var fraction: Double {
        guard total > 0, let s = finding.size_bytes else { return 0 }
        return Double(s) / Double(total)
    }

    private var fractionOfTotal: String {
        String(format: "%.1f%%", fraction * 100)
    }

    private var canTrash: Bool { isSafeForDisk(finding.target.value) }

    private var isDir: Bool { finding.title.contains("dir") }

    var body: some View {
        if isTrashed { EmptyView() } else {
            HStack(spacing: 12) {
                // Icon
                ZStack {
                    RoundedRectangle(cornerRadius: 7)
                        .fill(segmentColor.opacity(0.18))
                        .frame(width: 32, height: 32)
                    Image(systemName: isDir ? "folder.fill" : "doc.fill")
                        .font(.system(size: 14))
                        .foregroundStyle(segmentColor)
                }

                // Name + path
                VStack(alignment: .leading, spacing: 2) {
                    FilePathDisplay(
                        path: finding.target.value,
                        fontSize: 12,
                        pathFontSize: 9,
                        showIcon: false,
                        maxPathLines: 1
                    )
                }

                Spacer()

                // Size + fraction bar + size label
                VStack(alignment: .trailing, spacing: 4) {
                    HStack(spacing: 8) {
                        Text(fractionOfTotal)
                            .font(.system(size: 10))
                            .foregroundStyle(XTheme.textTertiary)
                        Text(finding.sizeFormatted)
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                            .frame(width: 72, alignment: .trailing)
                    }

                    GeometryReader { geo in
                        ZStack(alignment: .leading) {
                            Capsule()
                                .fill(XTheme.bgTertiary)
                                .frame(height: 4)
                            Capsule()
                                .fill(
                                    LinearGradient(
                                        colors: [segmentColor, segmentColor.opacity(0.5)],
                                        startPoint: .leading, endPoint: .trailing
                                    )
                                )
                                .frame(width: geo.size.width * CGFloat(min(fraction * 3, 1.0)), height: 4)
                        }
                    }
                    .frame(width: 120, height: 4)
                }
            }
            .padding(.vertical, 10)
            .contentShape(Rectangle())
            .contextMenu {
                Button {
                    revealInFinder(path: finding.target.value)
                } label: {
                    Label("Reveal in Finder", systemImage: "folder")
                }
                if canTrash {
                    Divider()
                    Button(role: .destructive) {
                        showTrashConfirm = true
                    } label: {
                        Label("Move to Trash", systemImage: "trash")
                    }
                }
            }
            .alert("Move to Trash?", isPresented: $showTrashConfirm) {
                Button("Cancel", role: .cancel) {}
                Button("Move to Trash", role: .destructive) { moveToTrash() }
            } message: {
                Text("\"\((finding.target.value as NSString).lastPathComponent)\" will be moved to the Trash.")
            }
        }
    }

    // Deterministic colour based on path so rows match the donut
    private var segmentColor: Color {
        let colors: [Color] = [
            XTheme.accent,
            XTheme.teal,
            XTheme.safe,
            XTheme.warning,
            XTheme.high,
            XTheme.accentBright,
            XTheme.anomaly,
        ]
        let h = abs(finding.target.value.hashValue) % colors.count
        return colors[h]
    }

    private func moveToTrash() {
        let url = URL(fileURLWithPath: finding.target.value)
        var resultingURL: NSURL?
        if (try? FileManager.default.trashItem(at: url, resultingItemURL: &resultingURL)) != nil {
            withAnimation { isTrashed = true }
        }
    }
}

// MARK: - isSafeForDisk helper (shared)

func isSafeForDisk(_ path: String) -> Bool {
    let home = FileManager.default.homeDirectoryForCurrentUser.path
    return path.hasPrefix(home + "/Library/") ||
           path.hasPrefix(home + "/Downloads/") ||
           path.hasPrefix("/private/tmp/")
}

// MARK: - Quick Clean Bar (kept from before)

struct DiskQuickCleanBar: View {
    let findings: [Finding]
    @State private var isShowingSheet = false

    private var safeFindings: [Finding] {
        findings.filter { isSafeForDisk($0.target.value) }
    }

    var body: some View {
        if !safeFindings.isEmpty {
            Button {
                isShowingSheet = true
            } label: {
                HStack(spacing: 8) {
                    Image(systemName: "trash")
                        .font(.system(size: 14, weight: .semibold))
                    Text("Clean Reclaimable Items")
                        .font(.system(size: 13, weight: .semibold))
                    Spacer()
                    Text(formatBytes(safeFindings.compactMap { $0.size_bytes }.reduce(0, +)))
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(XTheme.safe)
                    Image(systemName: "chevron.right")
                        .font(.system(size: 10))
                }
                .foregroundStyle(XTheme.textPrimary)
                .padding(.horizontal, 14)
                .padding(.vertical, 10)
                .background(XTheme.safe.opacity(0.12))
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(XTheme.safe.opacity(0.3), lineWidth: 1)
                )
                .clipShape(RoundedRectangle(cornerRadius: 10))
            }
            .buttonStyle(.plain)
            .sheet(isPresented: $isShowingSheet) {
                DiskQuickCleanSheet(findings: safeFindings)
            }
        }
    }
}

// MARK: - Quick Clean Sheet

struct DiskQuickCleanSheet: View {
    let findings: [Finding]
    @Environment(\.dismiss) private var dismiss
    @State private var trashed: Set<String> = []
    @State private var errorMessage: String?

    private var remaining: [Finding] {
        findings.filter { !trashed.contains($0.target.value) }
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                if let errorMessage {
                    HStack(spacing: 6) {
                        Image(systemName: "exclamationmark.triangle")
                        Text(errorMessage).lineLimit(2)
                        Spacer()
                    }
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.high)
                    .padding(12)
                    .background(XTheme.high.opacity(0.12))
                    .clipShape(RoundedRectangle(cornerRadius: 8))
                    .padding([.horizontal, .top])
                }

                List {
                    Section {
                        ForEach(remaining) { finding in
                            HStack(spacing: 12) {
                                VStack(alignment: .leading, spacing: 2) {
                                    Text((finding.target.value as NSString).lastPathComponent)
                                        .font(.system(size: 13, weight: .semibold))
                                        .foregroundStyle(XTheme.textPrimary)
                                        .lineLimit(1)
                                    Text(finding.target.value)
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.textTertiary)
                                        .lineLimit(1)
                                }
                                Spacer()
                                Text(finding.sizeFormatted)
                                    .font(.system(size: 12, weight: .semibold))
                                    .foregroundStyle(XTheme.accent)
                            }
                        }
                    } header: {
                        Text("\(remaining.count) safe item(s) · \(formatBytes(remaining.compactMap { $0.size_bytes }.reduce(0, +)))")
                    }
                }

                HStack(spacing: 12) {
                    Button("Cancel") { dismiss() }
                        .buttonStyle(.plain)
                        .foregroundStyle(XTheme.textSecondary)

                    Button {
                        moveAllToTrash()
                    } label: {
                        HStack(spacing: 6) {
                            Image(systemName: "trash")
                            Text("Move \(remaining.count) to Trash")
                        }
                        .font(.system(size: 13, weight: .semibold))
                        .foregroundStyle(.white)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 10)
                        .background(remaining.isEmpty ? XTheme.textTertiary : XTheme.high)
                        .clipShape(Capsule())
                    }
                    .buttonStyle(.plain)
                    .disabled(remaining.isEmpty)
                }
                .padding()
                .background(XTheme.sidebarGradient)
            }
            .background(XTheme.voidGradient)
            .navigationTitle("Clean Reclaimable Items")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Close") { dismiss() }
                        .foregroundStyle(XTheme.textSecondary)
                }
            }
        }
    }

    private func moveAllToTrash() {
        var failed = 0
        for finding in remaining {
            let url = URL(fileURLWithPath: finding.target.value)
            var resultingURL: NSURL?
            do {
                try FileManager.default.trashItem(at: url, resultingItemURL: &resultingURL)
                trashed.insert(finding.target.value)
            } catch {
                failed += 1
            }
        }
        if failed > 0 {
            errorMessage = "Could not move \(failed) item(s) to Trash."
        } else if remaining.isEmpty {
            dismiss()
        }
    }
}

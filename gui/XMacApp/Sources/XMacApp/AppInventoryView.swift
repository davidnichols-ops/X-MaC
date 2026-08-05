import SwiftUI

struct AppInventoryItem: Identifiable, Hashable {
    let id = UUID()
    let name: String
    let bundleID: String
    let version: String
    let path: String
    let sizeBytes: Int
    let relatedPaths: [RelatedPath]
    let isSystemApp: Bool
    let isRemovable: Bool
}

struct RelatedPath: Identifiable, Hashable {
    let id = UUID()
    let path: String
    let sizeBytes: Int
    let kind: RelatedPathKind
}

enum RelatedPathKind: String, CaseIterable {
    case container, cache, support, preference, savedState, groupContainer, other

    var label: String {
        switch self {
        case .container: return "Container"
        case .cache: return "Cache"
        case .support: return "Application Support"
        case .preference: return "Preferences"
        case .savedState: return "Saved State"
        case .groupContainer: return "Group Container"
        case .other: return "Other"
        }
    }

    var icon: String {
        switch self {
        case .container: return "cube.box"
        case .cache: return "clock.arrow.circlepath"
        case .support: return "folder"
        case .preference: return "slider.horizontal.3"
        case .savedState: return "bookmark"
        case .groupContainer: return "person.2"
        case .other: return "doc"
        }
    }
}

struct AppInventoryView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var apps: [AppInventoryItem] = []
    @State private var searchText: String = ""
    @State private var sortMode: AppSortMode = .size
    @State private var selectedAppIDs: Set<UUID> = []
    @State private var selectedRelatedPaths: Set<String> = []
    @State private var isLoading = false
    @State private var errorMessage: String?
    @State private var cleanupResults: [CleanupResult] = []
    @State private var showAppTrashConfirmation = false

    private var filteredApps: [AppInventoryItem] {
        let base = apps.filter { app in
            searchText.isEmpty || app.name.localizedCaseInsensitiveContains(searchText) || app.bundleID.localizedCaseInsensitiveContains(searchText)
        }
        switch sortMode {
        case .size:
            return base.sorted { $0.sizeBytes > $1.sizeBytes }
        case .name:
            return base.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        case .kind:
            return base.sorted { $0.isSystemApp == $1.isSystemApp ? $0.name < $1.name : !$0.isSystemApp }
        }
    }

    private var totalFootprint: Int { apps.map(\.sizeBytes).reduce(0, +) }
    private var selectedApps: [AppInventoryItem] { apps.filter { selectedAppIDs.contains($0.id) } }
    private var selectedApp: AppInventoryItem? { selectedApps.first }

    var body: some View {
        VStack(spacing: 0) {
            header
                .padding(20)

            Divider().background(XTheme.cardBorder)

            HStack(spacing: 0) {
                appList
                    .frame(minWidth: 320, idealWidth: 360)

                Divider().background(XTheme.cardBorder)

                detailPanel
                    .frame(minWidth: 420)
            }
        }
        .background(XTheme.voidGradient)
        .task { if apps.isEmpty && !isLoading { scan() } }
    }

    // MARK: - Header

    private var header: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(spacing: 16) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Applications")
                        .font(.system(size: 18, weight: .bold))
                        .foregroundStyle(XTheme.textPrimary)
                    Text("\(apps.count) apps discovered · total footprint \(formatBytes(totalFootprint))")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)
                }
                Spacer()

                Button(isLoading ? "Scanning…" : "Rescan") {
                    scan()
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .disabled(isLoading)

                Button {
                    showAppTrashConfirmation = true
                } label: {
                    Label("Move to Trash", systemImage: "trash")
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.danger)
                .controlSize(.small)
                .disabled(selectedApps.isEmpty || selectedApps.contains { !$0.isRemovable } || isLoading)
                .confirmationDialog("Move selected apps to Trash?", isPresented: $showAppTrashConfirmation, titleVisibility: .visible) {
                    Button("Move \(selectedApps.count) apps to Trash", role: .destructive) {
                        moveSelectedAppsToTrash()
                    }
                    Button("Cancel", role: .cancel) {}
                } message: {
                    Text("This moves the selected .app bundles to the Trash. It does not remove their support files automatically.")
                }
            }

            HStack(spacing: 12) {
                HStack(spacing: 6) {
                    Image(systemName: "magnifyingglass")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textTertiary)
                    TextField("Search by name or bundle ID", text: $searchText)
                        .font(.system(size: 12))
                        .textFieldStyle(.plain)
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 7)
                .background(XTheme.sidebarGradient)
                .clipShape(RoundedRectangle(cornerRadius: 8))

                HStack(spacing: 4) {
                    ForEach(AppSortMode.allCases) { mode in
                        Button(mode.label) {
                            sortMode = mode
                        }
                        .buttonStyle(.borderless)
                        .controlSize(.small)
                        .font(.system(size: 11, weight: sortMode == mode ? .semibold : .regular))
                        .foregroundStyle(sortMode == mode ? XTheme.accent : XTheme.textSecondary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(sortMode == mode ? XTheme.accent.opacity(0.12) : Color.clear)
                        .clipShape(RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }

    // MARK: - App List

    private var appList: some View {
        VStack(spacing: 0) {
            if let err = errorMessage {
                Spacer()
                XErrorState(
                    summary: "Failed to scan applications",
                    detail: err,
                    retryAction: { errorMessage = nil; scan() }
                )
                Spacer()
            } else if isLoading && apps.isEmpty {
                Spacer()
                VStack(spacing: 12) {
                    ProgressView()
                        .scaleEffect(0.8)
                    Text("Scanning applications…")
                        .font(.system(size: 12))
                        .foregroundStyle(XTheme.textSecondary)
                }
                Spacer()
            } else if filteredApps.isEmpty {
                Spacer()
                EmptyScanView(message: apps.isEmpty ? "No applications found." : "No apps match your search.")
                Spacer()
            } else {
                ScrollView {
                    LazyVStack(spacing: 2) {
                        ForEach(filteredApps) { app in
                            AppListRow(
                                app: app,
                                isSelected: selectedAppIDs.contains(app.id),
                                isPrimary: selectedApp?.id == app.id
                            ) {
                                toggleAppSelection(app)
                            }
                        }
                    }
                    .padding(12)
                }
            }
        }
        .background(XTheme.sidebarGradient)
    }

    // MARK: - Detail Panel

    private var detailPanel: some View {
        Group {
            if let app = selectedApp {
                AppDetailPanel(
                    app: app,
                    selectedRelatedPaths: $selectedRelatedPaths,
                    cleanupResults: cleanupResults,
                    onReveal: { revealInFinder(path: $0) },
                    onTrashRelated: { trashSelectedRelatedPaths() }
                )
            } else {
                VStack {
                    Spacer()
                    EmptyScanView(message: "Select one or more apps to inspect their footprint and support files.")
                    Spacer()
                }
                .background(XTheme.voidGradient)
            }
        }
    }

    // MARK: - Actions

    private func toggleAppSelection(_ app: AppInventoryItem) {
        if selectedAppIDs.contains(app.id) {
            selectedAppIDs.remove(app.id)
        } else {
            selectedAppIDs.insert(app.id)
        }
        selectedRelatedPaths.removeAll()
    }

    private func scan() {
        guard !isLoading else { return }
        isLoading = true
        errorMessage = nil
        selectedAppIDs.removeAll()
        selectedRelatedPaths.removeAll()
        cleanupResults.removeAll()
        Task {
            let found = await Self.collectApplications()
            await MainActor.run {
                apps = found
                isLoading = false
            }
        }
    }

    private func moveSelectedAppsToTrash() {
        let targets = selectedApps.filter(\.isRemovable)
        guard !targets.isEmpty else { return }
        Task { @MainActor in
            var results: [CleanupResult] = []
            for app in targets {
                do {
                    var resultingURL: NSURL?
                    try FileManager.default.trashItem(at: URL(fileURLWithPath: app.path), resultingItemURL: &resultingURL)
                    results.append(CleanupResult(path: app.path, success: true, message: "Moved to Trash"))
                } catch {
                    results.append(CleanupResult(path: app.path, success: false, message: error.localizedDescription))
                }
            }
            cleanupResults = results
            selectedAppIDs.subtract(targets.map(\.id))
            runner.cleanupMessage = "Moved \(results.filter(\.success).count) of \(results.count) apps to Trash."
        }
    }

    private func trashSelectedRelatedPaths() {
        let paths = Array(selectedRelatedPaths)
        guard !paths.isEmpty else { return }
        Task { @MainActor in
            var results: [CleanupResult] = []
            for path in paths {
                guard runner.isSafeCleanupPath(path) else {
                    results.append(CleanupResult(path: path, success: false, message: "Blocked: protected system path"))
                    continue
                }
                do {
                    var resultingURL: NSURL?
                    try FileManager.default.trashItem(at: URL(fileURLWithPath: path), resultingItemURL: &resultingURL)
                    results.append(CleanupResult(path: path, success: true, message: "Moved to Trash"))
                } catch {
                    results.append(CleanupResult(path: path, success: false, message: error.localizedDescription))
                }
            }
            cleanupResults = results
            selectedRelatedPaths.removeAll()
            runner.cleanupMessage = "Moved \(results.filter(\.success).count) of \(results.count) support files to Trash."
        }
    }

    private func revealInFinder(path: String) {
        NSWorkspace.shared.selectFile(path, inFileViewerRootedAtPath: "")
    }

    // MARK: - Collection

    private static func collectApplications() async -> [AppInventoryItem] {
        let dirs = ["/Applications", NSHomeDirectory() + "/Applications"]
        var items: [AppInventoryItem] = []
        let fm = FileManager.default
        for dir in dirs {
            guard let entries = try? fm.contentsOfDirectory(atPath: dir) else { continue }
            for entry in entries where entry.hasSuffix(".app") {
                let path = (dir as NSString).appendingPathComponent(entry)
                let info = readInfoPlist(path: path)
                let size = await directorySize(path: path)
                let related = relatedPaths(bundleID: info.bundleID, name: info.name)
                let isSystem = path.hasPrefix("/System")
                let isRemovable = !isSystem && (path.hasPrefix("/Applications/") || path.hasPrefix(NSHomeDirectory() + "/Applications/"))
                items.append(AppInventoryItem(
                    name: info.name,
                    bundleID: info.bundleID,
                    version: info.version,
                    path: path,
                    sizeBytes: size,
                    relatedPaths: related,
                    isSystemApp: isSystem,
                    isRemovable: isRemovable
                ))
            }
        }
        return items
    }

    private static func readInfoPlist(path: String) -> (name: String, bundleID: String, version: String) {
        let plist = (path as NSString).appendingPathComponent("Contents/Info.plist")
        guard let data = try? Data(contentsOf: URL(fileURLWithPath: plist)),
              let info = try? PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any] else {
            let name = (path as NSString).lastPathComponent.replacingOccurrences(of: ".app", with: "")
            return (name, "unknown", "unknown")
        }
        let name = info["CFBundleName"] as? String
            ?? info["CFBundleDisplayName"] as? String
            ?? (path as NSString).lastPathComponent.replacingOccurrences(of: ".app", with: "")
        let bundleID = info["CFBundleIdentifier"] as? String ?? "unknown"
        let version = info["CFBundleShortVersionString"] as? String
            ?? info["CFBundleVersion"] as? String
            ?? "unknown"
        return (name, bundleID, version)
    }

    private static func directorySize(path: String) async -> Int {
        await Task.detached {
            let fm = FileManager.default
            guard let enumerator = fm.enumerator(atPath: path) else { return 0 }
            var total: Int = 0
            let base = URL(fileURLWithPath: path)
            var count = 0
            while let file = enumerator.nextObject() as? String {
                let url = base.appendingPathComponent(file)
                if let attr = try? fm.attributesOfItem(atPath: url.path),
                   let size = attr[.size] as? Int {
                    total += size
                }
                count += 1
                // Yield periodically to keep the UI responsive during large bundles.
                if count % 500 == 0 {
                    await Task.yield()
                }
            }
            return total
        }.value
    }

    private static func relatedPaths(bundleID: String, name: String) -> [RelatedPath] {
        let home = NSHomeDirectory()
        let candidates: [(String, RelatedPathKind)] = [
            ("\(home)/Library/Application Support/\(name)", .support),
            ("\(home)/Library/Application Support/\(bundleID)", .support),
            ("\(home)/Library/Caches/\(name)", .cache),
            ("\(home)/Library/Caches/\(bundleID)", .cache),
            ("\(home)/Library/Preferences/\(bundleID).plist", .preference),
            ("\(home)/Library/Saved Application State/\(bundleID).savedState", .savedState),
            ("\(home)/Library/Containers/\(bundleID)", .container),
            ("\(home)/Library/Group Containers/\(bundleID)", .groupContainer),
        ]
        let fm = FileManager.default
        return candidates.compactMap { path, kind in
            guard fm.fileExists(atPath: path) else { return nil }
            let size = (try? fm.attributesOfItem(atPath: path)[.size] as? Int) ?? 0
            return RelatedPath(path: path, sizeBytes: size, kind: kind)
        }
    }
}

enum AppSortMode: String, CaseIterable, Identifiable {
    case size, name, kind
    var id: String { rawValue }
    var label: String {
        switch self {
        case .size: return "Size"
        case .name: return "Name"
        case .kind: return "Kind"
        }
    }
}

// MARK: - App List Row

struct AppListRow: View {
    let app: AppInventoryItem
    let isSelected: Bool
    let isPrimary: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 14))
                    .foregroundStyle(isSelected ? XTheme.safe : XTheme.textTertiary)
                    .frame(width: 20)

                Image(systemName: app.isSystemApp ? "gear.circle" : "app.badge")
                    .font(.system(size: 16))
                    .foregroundStyle(app.isSystemApp ? XTheme.textTertiary : XTheme.accent)
                    .frame(width: 20)

                VStack(alignment: .leading, spacing: 2) {
                    Text(app.name)
                        .font(.system(size: 12, weight: isPrimary ? .semibold : .medium))
                        .foregroundStyle(XTheme.textPrimary)
                        .lineLimit(1)
                    Text(app.bundleID)
                        .font(.system(size: 9))
                        .foregroundStyle(XTheme.textTertiary)
                        .lineLimit(1)
                }

                Spacer()

                Text(formatBytes(app.sizeBytes))
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(isPrimary ? XTheme.accent.opacity(0.12) : (isSelected ? XTheme.safe.opacity(0.08) : Color.clear))
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

// MARK: - Detail Panel

struct AppDetailPanel: View {
    let app: AppInventoryItem
    @Binding var selectedRelatedPaths: Set<String>
    let cleanupResults: [CleanupResult]
    let onReveal: (String) -> Void
    let onTrashRelated: () -> Void
    @State private var showRelatedTrashConfirmation = false

    private var relatedTotal: Int { app.relatedPaths.map(\.sizeBytes).reduce(0, +) }
    private var selectedRelatedTotal: Int {
        app.relatedPaths.filter { selectedRelatedPaths.contains($0.path) }.map(\.sizeBytes).reduce(0, +)
    }
    private var selectedRelatedCount: Int { selectedRelatedPaths.intersection(app.relatedPaths.map(\.path)).count }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                HStack(spacing: 16) {
                    Image(systemName: app.isSystemApp ? "gear.circle.fill" : "app.badge.fill")
                        .font(.system(size: 48))
                        .foregroundStyle(app.isSystemApp ? XTheme.textTertiary : XTheme.accent)
                    VStack(alignment: .leading, spacing: 6) {
                        Text(app.name).font(.system(size: 22, weight: .bold)).foregroundStyle(XTheme.textPrimary)
                        Text(app.bundleID).font(.system(size: 12)).foregroundStyle(XTheme.textSecondary).lineLimit(1)
                        Text("Version \(app.version) · \(app.isSystemApp ? "System" : "User") app")
                            .font(.system(size: 12))
                            .foregroundStyle(XTheme.textTertiary)
                    }
                    Spacer()
                }

                HStack(spacing: 12) {
                    XCard {
                        VStack(alignment: .leading, spacing: 10) {
                            XSectionHeader(title: "Bundle", icon: "ruler")
                            Text(formatBytes(app.sizeBytes))
                                .font(.system(size: 28, weight: .bold))
                                .foregroundStyle(XTheme.metallicGradient)
                                .xGlow(XTheme.accent, radius: 4)
                            FilePathDisplay(path: app.path, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                        }
                    }
                    .frame(maxWidth: .infinity)

                    XCard {
                        VStack(alignment: .leading, spacing: 10) {
                            XSectionHeader(title: "Support Files", icon: "folder.badge.plus")
                            Text(formatBytes(relatedTotal))
                                .font(.system(size: 28, weight: .bold))
                                .foregroundStyle(XTheme.metallicGradient)
                                .xGlow(XTheme.accent, radius: 4)
                            Text("\(app.relatedPaths.count) related paths")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                    }
                    .frame(maxWidth: .infinity)
                }

                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            XSectionHeader(title: "Related Paths", icon: "folder.badge.plus", count: app.relatedPaths.count)
                            Spacer()
                            if !app.relatedPaths.isEmpty {
                                Button("Select all") {
                                    selectedRelatedPaths.formUnion(app.relatedPaths.map(\.path))
                                }
                                .buttonStyle(.borderless)
                                .controlSize(.small)
                                Button("Clear") {
                                    selectedRelatedPaths.subtract(app.relatedPaths.map(\.path))
                                }
                                .buttonStyle(.borderless)
                                .controlSize(.small)
                            }
                        }

                        if app.relatedPaths.isEmpty {
                            Text("No related support paths detected.")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                        } else {
                            LazyVStack(spacing: 6) {
                                ForEach(app.relatedPaths) { related in
                                    RelatedPathRow(
                                        related: related,
                                        isSelected: selectedRelatedPaths.contains(related.path)
                                    ) {
                                        if selectedRelatedPaths.contains(related.path) {
                                            selectedRelatedPaths.remove(related.path)
                                        } else {
                                            selectedRelatedPaths.insert(related.path)
                                        }
                                    } onReveal: {
                                        onReveal(related.path)
                                    }
                                }
                            }

                            HStack {
                                Spacer()
                                Text("Selected: \(formatBytes(selectedRelatedTotal)) from \(selectedRelatedCount) paths")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                                Button {
                                    showRelatedTrashConfirmation = true
                                } label: {
                                    Label("Move to Trash", systemImage: "trash")
                                }
                                .buttonStyle(.borderedProminent)
                                .tint(XTheme.warning)
                                .controlSize(.small)
                                .disabled(selectedRelatedCount == 0)
                                .confirmationDialog("Move support files to Trash?", isPresented: $showRelatedTrashConfirmation, titleVisibility: .visible) {
                                    Button("Move \(selectedRelatedCount) support files to Trash", role: .destructive) {
                                        onTrashRelated()
                                    }
                                    Button("Cancel", role: .cancel) {}
                                } message: {
                                    Text("These support files, caches, and preferences will be moved to macOS Trash. The app itself is not affected.")
                                }
                            }
                        }
                    }
                }

                if !cleanupResults.isEmpty {
                    XCard {
                        VStack(alignment: .leading, spacing: 10) {
                            XSectionHeader(title: "Cleanup Results", icon: "checklist", count: cleanupResults.count)
                            ForEach(cleanupResults) { result in
                                HStack {
                                    Image(systemName: result.success ? "checkmark.circle.fill" : "xmark.circle.fill")
                                        .foregroundStyle(result.success ? XTheme.safe : XTheme.danger)
                                    Text((result.path as NSString).lastPathComponent)
                                        .font(.system(size: 11))
                                        .foregroundStyle(XTheme.textPrimary)
                                        .lineLimit(1)
                                    Spacer()
                                    Text(result.message)
                                        .font(.system(size: 10))
                                        .foregroundStyle(result.success ? XTheme.safe : XTheme.danger)
                                }
                            }
                        }
                    }
                }

                if app.isRemovable {
                    Text("Tap \"Move to Trash\" in the toolbar to remove this app bundle.")
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textTertiary)
                } else {
                    Text("System apps and apps outside /Applications or ~/Applications cannot be removed from this view.")
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }
}

// MARK: - Related Path Row

struct RelatedPathRow: View {
    let related: RelatedPath
    let isSelected: Bool
    let onToggle: () -> Void
    let onReveal: () -> Void

    var body: some View {
        Button(action: onToggle) {
            HStack(spacing: 10) {
                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 13))
                    .foregroundStyle(isSelected ? XTheme.safe : XTheme.textTertiary)
                    .frame(width: 20)

                Image(systemName: related.kind.icon)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.info)
                    .frame(width: 20)

                VStack(alignment: .leading, spacing: 2) {
                    HStack {
                        FilePathDisplay(path: related.path, fontSize: 12, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                        Spacer()
                        Text(formatBytes(related.sizeBytes))
                            .font(.system(size: 11, weight: .medium))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    Text(related.kind.label)
                        .font(.system(size: 9, weight: .medium))
                        .foregroundStyle(XTheme.textTertiary)
                        .padding(.horizontal, 6)
                        .padding(.vertical, 1)
                        .background(XTheme.bgTertiary)
                        .clipShape(Capsule())
                }
                .contentShape(Rectangle())
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(.plain)
        .contextMenu {
            Button("Reveal in Finder") {
                onReveal()
            }
        }
    }
}

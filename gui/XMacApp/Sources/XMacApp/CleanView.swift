import SwiftUI

struct CleanView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var searchText: String = ""
    @State private var selectedCategories: Set<String> = []
    @State private var showingExportPanel = false

    private var cleanFindings: [Finding] {
        runner.findings.filter { $0.engine == "clean" }
    }

    private var filteredFindings: [Finding] {
        let base = searchText.isEmpty ? cleanFindings : cleanFindings.filter {
            $0.title.localizedCaseInsensitiveContains(searchText) ||
            $0.target.value.localizedCaseInsensitiveContains(searchText)
        }
        if selectedCategories.isEmpty { return base }
        return base.filter { selectedCategories.contains($0.category) }
    }

    private var totalReclaimable: Int {
        cleanFindings.compactMap { $0.size_bytes }.reduce(0, +)
    }

    private var selectedReclaimable: Int {
        cleanFindings
            .filter { runner.selectedPaths.contains($0.target.value) }
            .compactMap { $0.size_bytes }
            .reduce(0, +)
    }

    private var categoryBreakdown: [(String, Int, Int)] {
        let grouped = Dictionary(grouping: cleanFindings, by: { $0.category })
        return grouped.map { (cat, items) in
            (cat, items.count, items.compactMap { $0.size_bytes }.reduce(0, +))
        }.sorted { $0.2 > $1.2 }
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                if runner.isScanning {
                    ScanProgressView(phase: runner.scanPhase)
                } else if let err = runner.error {
                    ScanErrorView(message: err)
                } else if cleanFindings.isEmpty {
                    EmptyScanView(message: "No reclaimable space found. Run a clean scan to see results.")
                } else {
                    CleanReclaimCard(
                        reclaimable: totalReclaimable,
                        selectedReclaimable: selectedReclaimable,
                        count: cleanFindings.count,
                        selectedCount: runner.selectedPaths.count
                    )

                    CleanCategoryBreakdown(
                        breakdown: categoryBreakdown,
                        total: totalReclaimable,
                        selectedCategories: $selectedCategories
                    )

                    SearchBar(text: $searchText)
                    CleanupToolbar()
                    CleanFindingsList(findings: filteredFindings)

                    if let message = runner.cleanupMessage {
                        Text(message)
                            .font(.system(size: 12, weight: .medium))
                            .foregroundStyle(XTheme.safe)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
            }
            .padding(20)
        }
        .background(XTheme.bgPrimary)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button("Export Report") {
                    showingExportPanel = true
                }
                .disabled(runner.findings.isEmpty)
            }
        }
        .fileExporter(
            isPresented: $showingExportPanel,
            document: XMacReportDocument(findings: runner.findings, gnnScores: runner.gnnScores),
            contentType: .plainText,
            defaultFilename: "XMaC-Report"
        ) { result in
            if case .failure(let error) = result {
                runner.error = error.localizedDescription
            }
        }
    }
}

// MARK: - Reclaim Card

struct CleanReclaimCard: View {
    let reclaimable: Int
    let selectedReclaimable: Int
    let count: Int
    let selectedCount: Int

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Reclaimable Space", icon: "trash.circle", count: count)

                HStack(spacing: 16) {
                    Image(systemName: "arrow.down.circle.fill")
                        .font(.system(size: 36))
                        .foregroundStyle(XTheme.safe)
                        .xGlow(XTheme.safe)

                    VStack(alignment: .leading, spacing: 4) {
                        Text(formatBytes(reclaimable))
                            .font(.system(size: 28, weight: .bold))
                            .foregroundStyle(XTheme.textPrimary)
                        Text("\(count) items can be safely removed")
                            .font(.system(size: 12))
                            .foregroundStyle(XTheme.textSecondary)
                        if selectedCount > 0 {
                            Text("Selected: \(formatBytes(selectedReclaimable)) from \(selectedCount) items")
                                .font(.system(size: 11, weight: .semibold))
                                .foregroundStyle(XTheme.accent)
                        }
                    }

                    Spacer()
                }
            }
        }
    }
}

// MARK: - Category Breakdown

struct CleanCategoryBreakdown: View {
    let breakdown: [(String, Int, Int)]
    let total: Int
    @Binding var selectedCategories: Set<String>

    var body: some View {
        XCard {
            VStack(alignment: .leading, spacing: 10) {
                XSectionHeader(title: "Category Breakdown", icon: "square.grid.2x2")

                LazyVStack(spacing: 8) {
                    ForEach(breakdown, id: \.0) { category, count, size in
                        let isSelected = selectedCategories.contains(category)
                        Button {
                            if isSelected {
                                selectedCategories.remove(category)
                            } else {
                                selectedCategories.insert(category)
                            }
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                                    .font(.system(size: 12))
                                    .foregroundStyle(isSelected ? XTheme.accent : XTheme.textTertiary)
                                    .frame(width: 18)
                                Text(category.replacingOccurrences(of: "_", with: " ").capitalized)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                    .frame(width: 120, alignment: .leading)

                                GeometryReader { geo in
                                    let ratio = total > 0 ? CGFloat(size) / CGFloat(total) : 0
                                    ZStack(alignment: .leading) {
                                        RoundedRectangle(cornerRadius: 4)
                                            .fill(XTheme.bgTertiary)
                                            .frame(height: 8)
                                        RoundedRectangle(cornerRadius: 4)
                                            .fill(isSelected ? XTheme.accent : XTheme.textTertiary)
                                            .frame(width: geo.size.width * ratio, height: 8)
                                    }
                                }
                                .frame(height: 8)

                                Text(formatBytes(size))
                                    .font(.system(size: 11, weight: .medium))
                                    .foregroundStyle(XTheme.textSecondary)
                                    .frame(width: 70, alignment: .trailing)

                                Text("\(count)")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textTertiary)
                                    .frame(width: 30, alignment: .trailing)
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }
}

// MARK: - Findings List

struct CleanupToolbar: View {
    @EnvironmentObject var runner: XMacRunner
    @EnvironmentObject var settings: AppSettings
    @State private var showingConfirmation = false

    var body: some View {
        XCard {
            HStack(spacing: 10) {
                Image(systemName: "shield.checkered").foregroundStyle(XTheme.safe)
                VStack(alignment: .leading, spacing: 3) {
                    Text("Review before cleanup").font(.system(size: 12, weight: .semibold)).foregroundStyle(XTheme.textPrimary)
                    Text("Selected items are moved to macOS Trash, never permanently deleted.").font(.system(size: 10)).foregroundStyle(XTheme.textSecondary)
                }
                Spacer()
                Button("Select safe items") { runner.selectAllSafeFindings() }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(runner.isScanning)
                Button {
                    if settings.confirmCleanup {
                        showingConfirmation = true
                    } else {
                        runner.trashSelected()
                    }
                } label: {
                    Label(runner.isCleaning ? "Cleaning…" : "Move to Trash (\(runner.selectedPaths.count))", systemImage: "trash")
                }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.safe)
                .controlSize(.small)
                .disabled(runner.selectedPaths.isEmpty || runner.isCleaning || runner.isScanning)
                .confirmationDialog("Move selected items to Trash?", isPresented: $showingConfirmation, titleVisibility: .visible) {
                    Button("Move \(runner.selectedPaths.count) items to Trash", role: .destructive) {
                        runner.trashSelected()
                    }
                    Button("Cancel", role: .cancel) {}
                } message: {
                    Text("X-MaC will not permanently delete these items. They can be restored from the macOS Trash.")
                }
                Button("Clear") { runner.clearSelection() }
                    .buttonStyle(.borderless)
                    .controlSize(.small)
                    .disabled(runner.selectedPaths.isEmpty || runner.isScanning)
            }
        }
    }
}

struct CleanFindingsList: View {
    @EnvironmentObject var runner: XMacRunner
    let findings: [Finding]

    private var grouped: [(String, [Finding])] {
        Dictionary(grouping: findings, by: { $0.category })
            .sorted { $0.value.compactMap { $0.size_bytes }.reduce(0, +) > $1.value.compactMap { $0.size_bytes }.reduce(0, +) }
    }

    var body: some View {
        LazyVStack(spacing: 16) {
            ForEach(grouped, id: \.0) { category, items in
                XCard {
                    VStack(alignment: .leading, spacing: 10) {
                        XSectionHeader(
                            title: category.replacingOccurrences(of: "_", with: " ").capitalized,
                            icon: "folder",
                            count: items.count
                        )

                        LazyVStack(spacing: 8) {
                            ForEach(items) { finding in
                                SelectableFindingRow(finding: finding)
                            }
                        }
                    }
                }
            }
        }
    }
}

import Foundation
import SwiftUI

// MARK: - Resource Mode

/// Controls scan parallelism and CPU strain.
/// eco = sequential (lowest CPU), balanced = limited parallelism, turbo = full parallelism.
enum ResourceMode: String, CaseIterable, Identifiable {
    case eco
    case balanced
    case turbo

    var id: String { rawValue }

    var label: String {
        switch self {
        case .eco: return "Eco"
        case .balanced: return "Balanced"
        case .turbo: return "Turbo"
        }
    }

    var description: String {
        switch self {
        case .eco: return "Sequential — lowest CPU strain, slower scans"
        case .balanced: return "Limited parallelism — good balance of speed and CPU"
        case .turbo: return "Full parallelism — fastest scans, higher CPU"
        }
    }

    var icon: String {
        switch self {
        case .eco: return "leaf.fill"
        case .balanced: return "scalemass.fill"
        case .turbo: return "bolt.fill"
        }
    }
}

// MARK: - Clean Scan Scope

/// Defines which categories a clean scan should target.
/// Maps to the `--only` CLI flag for targeted scans.
enum CleanScanScope: String, CaseIterable, Identifiable {
    case all
    case orphanFile
    case packageCaches
    case buildArtifacts
    case caches
    case tempFiles
    case xcodeArtifacts
    case browserCaches
    case logs
    case trashBin
    case largeFiles
    case docker
    case duplicates

    var id: String { rawValue }

    var label: String {
        switch self {
        case .all: return "All Categories"
        case .orphanFile: return "Orphan Files Only"
        case .packageCaches: return "Package Caches Only"
        case .buildArtifacts: return "Build Artifacts Only"
        case .caches: return "Caches Only"
        case .tempFiles: return "Temp Files Only"
        case .xcodeArtifacts: return "Xcode Artifacts Only"
        case .browserCaches: return "Browser Caches Only"
        case .logs: return "Rotated Logs Only"
        case .trashBin: return "Trash Bins Only"
        case .largeFiles: return "Large Files Only"
        case .docker: return "Docker Caches Only"
        case .duplicates: return "Duplicates Only"
        }
    }

    var icon: String {
        switch self {
        case .all: return "square.grid.2x2.fill"
        case .orphanFile: return "app.dashed"
        case .packageCaches: return "shippingbox.fill"
        case .buildArtifacts: return "hammer.fill"
        case .caches: return "internaldrive.fill"
        case .tempFiles: return "clock.arrow.circlepath"
        case .xcodeArtifacts: return "app.fill"
        case .browserCaches: return "globe.badge.chevron.backward"
        case .logs: return "doc.text.fill"
        case .trashBin: return "trash.fill"
        case .largeFiles: return "arrow.up.circle.fill"
        case .docker: return "cube.box.fill"
        case .duplicates: return "doc.on.doc.fill"
        }
    }

    /// The CLI `--only` category key, or nil for "all" (no --only flag).
    var cliKey: String? {
        switch self {
        case .all: return nil
        case .orphanFile: return "orphan_file"
        case .packageCaches: return "package_manager_cache"
        case .buildArtifacts: return "build_artifact"
        case .caches: return "cache"
        case .tempFiles: return "temp_file"
        case .xcodeArtifacts: return "xcode_artifact"
        case .browserCaches: return "browser_cache"
        case .logs: return "log"
        case .trashBin: return "trash_bin"
        case .largeFiles: return "large_file"
        case .docker: return "docker"
        case .duplicates: return "duplicate_file"
        }
    }

    var description: String {
        switch self {
        case .all: return "Scan all enabled cleanup categories"
        case .orphanFile: return "Find orphaned app support dirs from uninstalled apps"
        case .packageCaches: return "npm, pip, cargo, Homebrew, go caches — safe to clear"
        case .buildArtifacts: return "node_modules, target, __pycache__, dist, .pyc, .o"
        case .caches: return "General cache directories"
        case .tempFiles: return "DS_Store, editor swap files, /tmp contents"
        case .xcodeArtifacts: return "DerivedData, Archives, iOS DeviceSupport"
        case .browserCaches: return "Safari, Chrome, Firefox, Edge, Brave, Arc"
        case .logs: return "Rotated/archived log files (*.gz, *.0, etc.)"
        case .trashBin: return "Trash bins on all volumes"
        case .largeFiles: return "Files >= 100MB"
        case .docker: return "Docker image and build caches"
        case .duplicates: return "Duplicate files via BLAKE3 hashing (slower)"
        }
    }
}

@MainActor
final class AppSettings: ObservableObject {
    @AppStorage("xmac.appearance") var appearance = "system"
    @AppStorage("xmac.defaultScanMode") var defaultScanMode = "smart"
    @AppStorage("xmac.minimumAgeDays") var minimumAgeDays = 30
    @AppStorage("xmac.minimumSizeMB") var minimumSizeMB = 1
    @AppStorage("xmac.includeHidden") var includeHidden = false
    @AppStorage("xmac.followSymlinks") var followSymlinks = false
    @AppStorage("xmac.neuralMaxNodes") var neuralMaxNodes = 2000
    @AppStorage("xmac.trashInsteadOfDelete") var trashInsteadOfDelete = true
    @AppStorage("xmac.confirmCleanup") var confirmCleanup = true
    @AppStorage("xmac.excludePaths") var excludePaths = ""
    @AppStorage("xmac.resourceMode") var resourceMode = "balanced"
    @AppStorage("xmac.gnnAutoScore") var gnnAutoScore = false

    var resourceModeEnum: ResourceMode {
        get { ResourceMode(rawValue: resourceMode) ?? .balanced }
        set { resourceMode = newValue.rawValue }
    }

    var exclusionList: [String] {
        excludePaths
            .split(separator: "\n")
            .map(String.init)
            .filter { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    }
}

struct CleanupResult: Identifiable, Hashable {
    let id = UUID()
    let path: String
    let success: Bool
    let message: String
}

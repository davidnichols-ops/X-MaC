import Foundation

enum CleanupAction: String, Codable, CaseIterable, Identifiable {
    case trash, review, blocked
    var id: String { rawValue }

    var label: String {
        switch self {
        case .trash: return "Move to Trash"
        case .review: return "Review required"
        case .blocked: return "Never clean"
        }
    }

    var color: String {
        switch self {
        case .trash: return "safe"
        case .review: return "warning"
        case .blocked: return "danger"
        }
    }
}

struct CleanupProfile: Identifiable, Codable, Equatable {
    var id = UUID()
    var name: String
    var description: String
    var isBuiltIn: Bool
    var minimumAgeDays: Int
    var minimumSizeMB: Int
    var includeHidden: Bool
    var followSymlinks: Bool
    var confirmCleanup: Bool
    var trashInsteadOfDelete: Bool
    var categoryActions: [String: CleanupAction]
    var excludedPaths: [String]

    static let defaultProfile: CleanupProfile = .safe

    static let safe = CleanupProfile(
        name: "Safe Everyday",
        description: "Conservative cleanup of caches, temp files, and build artifacts only.",
        isBuiltIn: true,
        minimumAgeDays: 30,
        minimumSizeMB: 1,
        includeHidden: false,
        followSymlinks: false,
        confirmCleanup: true,
        trashInsteadOfDelete: true,
        categoryActions: [
            "cache": .trash,
            "temp_file": .trash,
            "build_artifact": .trash,
            "package_manager_cache": .trash,
            "browser_cache": .trash,
            "log": .trash,
            "trash_bin": .trash,
            "document_version": .trash,
            "language_file": .trash,
            "xcode_artifact": .trash,
            "orphan_file": .review,
            "mail_attachment": .review,
            "ios_backup": .review,
            "large_file": .review,
            "duplicate_file": .review,
            "universal_binary": .review,
            "system_info": .blocked,
            "system_maintenance": .blocked,
            "installed_app": .blocked,
        ],
        excludedPaths: []
    )

    static let developer = CleanupProfile(
        name: "Developer",
        description: "Aggressive cleanup of build outputs, package caches, and simulator data.",
        isBuiltIn: true,
        minimumAgeDays: 7,
        minimumSizeMB: 1,
        includeHidden: true,
        followSymlinks: false,
        confirmCleanup: true,
        trashInsteadOfDelete: true,
        categoryActions: [
            "cache": .trash,
            "temp_file": .trash,
            "build_artifact": .trash,
            "package_manager_cache": .trash,
            "browser_cache": .trash,
            "log": .trash,
            "trash_bin": .trash,
            "document_version": .trash,
            "language_file": .trash,
            "xcode_artifact": .trash,
            "orphan_file": .review,
            "large_file": .review,
            "duplicate_file": .review,
            "system_info": .blocked,
            "system_maintenance": .blocked,
            "installed_app": .blocked,
        ],
        excludedPaths: [
            "~/Projects",
            "~/.ssh",
        ]
    )

    static let creative = CleanupProfile(
        name: "Creative Professional",
        description: "Focus on render caches, preview files, and large media scratch data.",
        isBuiltIn: true,
        minimumAgeDays: 14,
        minimumSizeMB: 10,
        includeHidden: false,
        followSymlinks: false,
        confirmCleanup: true,
        trashInsteadOfDelete: true,
        categoryActions: [
            "cache": .trash,
            "temp_file": .trash,
            "document_version": .trash,
            "xcode_artifact": .review,
            "large_file": .review,
            "system_info": .blocked,
            "system_maintenance": .blocked,
            "installed_app": .blocked,
        ],
        excludedPaths: [
            "~/Movies",
            "~/Music",
        ]
    )

    static let maximumReclaim = CleanupProfile(
        name: "Maximum Reclaim",
        description: "Review every category for maximum space recovery. Use with caution.",
        isBuiltIn: true,
        minimumAgeDays: 30,
        minimumSizeMB: 1,
        includeHidden: true,
        followSymlinks: false,
        confirmCleanup: true,
        trashInsteadOfDelete: true,
        categoryActions: [
            "cache": .trash,
            "temp_file": .trash,
            "build_artifact": .trash,
            "package_manager_cache": .trash,
            "browser_cache": .trash,
            "log": .trash,
            "trash_bin": .trash,
            "document_version": .trash,
            "language_file": .trash,
            "xcode_artifact": .trash,
            "orphan_file": .review,
            "mail_attachment": .review,
            "ios_backup": .review,
            "large_file": .review,
            "duplicate_file": .review,
            "universal_binary": .review,
            "system_info": .blocked,
            "system_maintenance": .blocked,
            "installed_app": .blocked,
        ],
        excludedPaths: []
    )

    static var builtInProfiles: [CleanupProfile] {
        [.safe, .developer, .creative, .maximumReclaim]
    }

    func action(for category: String) -> CleanupAction {
        categoryActions[category] ?? .review
    }

    mutating func setAction(_ action: CleanupAction, for category: String) {
        categoryActions[category] = action
    }
}

@MainActor
final class ProfileStore: ObservableObject {
    @Published var profiles: [CleanupProfile] = CleanupProfile.builtInProfiles
    @Published var selectedProfileId: UUID = CleanupProfile.safe.id

    private let savePath: URL

    init() {
        let home = FileManager.default.homeDirectoryForCurrentUser
        savePath = home
            .appendingPathComponent("Library/Caches/com.xmac.gui")
            .appendingPathComponent("profiles.json")
        load()
    }

    var selectedProfile: CleanupProfile {
        get {
            profiles.first { $0.id == selectedProfileId } ?? CleanupProfile.safe
        }
        set {
            if let index = profiles.firstIndex(where: { $0.id == newValue.id }) {
                profiles[index] = newValue
                selectedProfileId = newValue.id
                save()
            }
        }
    }

    func add(_ profile: CleanupProfile) {
        profiles.append(profile)
        selectedProfileId = profile.id
        save()
    }

    func delete(_ profile: CleanupProfile) {
        guard !profile.isBuiltIn else { return }
        profiles.removeAll { $0.id == profile.id }
        if selectedProfileId == profile.id {
            selectedProfileId = CleanupProfile.safe.id
        }
        save()
    }

    func duplicate(_ profile: CleanupProfile) -> CleanupProfile {
        var copy = profile
        copy.id = UUID()
        copy.name = "\(profile.name) Copy"
        copy.isBuiltIn = false
        return copy
    }

    func exportSelected() -> String? {
        let encoder = JSONEncoder()
        encoder.outputFormatting = .prettyPrinted
        if let data = try? encoder.encode(selectedProfile) {
            return String(data: data, encoding: .utf8)
        }
        return nil
    }

    func `import`(_ json: String) -> Bool {
        guard let data = json.data(using: .utf8),
              let profile = try? JSONDecoder().decode(CleanupProfile.self, from: data) else {
            return false
        }
        var imported = profile
        imported.id = UUID()
        imported.isBuiltIn = false
        profiles.append(imported)
        selectedProfileId = imported.id
        save()
        return true
    }

    private func load() {
        guard FileManager.default.fileExists(atPath: savePath.path),
              let data = try? Data(contentsOf: savePath),
              let decoded = try? JSONDecoder().decode([CleanupProfile].self, from: data),
              !decoded.isEmpty else {
            return
        }
        profiles = decoded
        if !profiles.contains(where: { $0.id == selectedProfileId }) {
            selectedProfileId = profiles.first?.id ?? CleanupProfile.safe.id
        }
    }

    private func save() {
        try? FileManager.default.createDirectory(at: savePath.deletingLastPathComponent(), withIntermediateDirectories: true)
        if let data = try? JSONEncoder().encode(profiles) {
            try? data.write(to: savePath)
        }
    }
}

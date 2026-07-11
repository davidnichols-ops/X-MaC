import Foundation
import SwiftUI

@MainActor
final class AppSettings: ObservableObject {
    @AppStorage("xmac.appearance") var appearance = "system"
    @AppStorage("xmac.defaultScanMode") var defaultScanMode = "smart"
    @AppStorage("xmac.minimumAgeDays") var minimumAgeDays = 30
    @AppStorage("xmac.minimumSizeMB") var minimumSizeMB = 1
    @AppStorage("xmac.includeHidden") var includeHidden = false
    @AppStorage("xmac.followSymlinks") var followSymlinks = false
    @AppStorage("xmac.neuralMaxNodes") var neuralMaxNodes = 600
    @AppStorage("xmac.trashInsteadOfDelete") var trashInsteadOfDelete = true
    @AppStorage("xmac.confirmCleanup") var confirmCleanup = true
    @AppStorage("xmac.excludePaths") var excludePaths = ""

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

import SwiftUI

struct PerCategoryPolicyEditor: View {
    @Binding var profile: CleanupProfile

    private let categories = [
        "cache", "temp_file", "build_artifact", "package_manager_cache",
        "browser_cache", "log", "trash_bin", "document_version", "language_file",
        "xcode_artifact", "orphan_file", "mail_attachment", "ios_backup",
        "large_file", "duplicate_file", "universal_binary", "system_info",
        "system_maintenance", "installed_app"
    ]

    var body: some View {
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 10) {
            ForEach(categories, id: \.self) { category in
                HStack {
                    Text(category.replacingOccurrences(of: "_", with: " ").capitalized)
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textPrimary)
                    Spacer()
                    Picker("", selection: Binding(
                        get: { profile.action(for: category) },
                        set: { profile.setAction($0, for: category) }
                    )) {
                        ForEach(CleanupAction.allCases) { action in
                            Text(action.label).tag(action)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(width: 180)
                }
            }
        }
    }
}

struct ImportProfileSheet: View {
    @Binding var importJSON: String
    @ObservedObject var profiles: ProfileStore
    @Binding var isPresented: Bool
    @State private var error: String?

    var body: some View {
        VStack(spacing: 16) {
            Text("Import Profile").font(.headline)
            Text("Paste exported JSON below:")
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textSecondary)
            TextEditor(text: $importJSON)
                .font(.system(size: 11, design: .monospaced))
                .frame(minHeight: 150)
                .padding(6)
                .background(XTheme.bgTertiary)
                .clipShape(RoundedRectangle(cornerRadius: 7))
            if let error = error {
                Text(error).foregroundStyle(XTheme.danger).font(.system(size: 11))
            }
            HStack {
                Button("Cancel") { isPresented = false }
                Button("Import") {
                    if profiles.import(importJSON) {
                        isPresented = false
                    } else {
                        error = "Invalid profile JSON"
                    }
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(20)
        .frame(width: 420)
    }
}

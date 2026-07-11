import SwiftUI

struct SelectableFindingRow: View {
    @EnvironmentObject var runner: XMacRunner
    let finding: Finding

    private var path: String { finding.target.value }
    private var isSelected: Bool { runner.selectedPaths.contains(path) }

    var body: some View {
        Button {
            runner.toggleSelection(path: path)
        } label: {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: isSelected ? "checkmark.square.fill" : "square")
                    .font(.system(size: 16))
                    .foregroundStyle(isSelected ? XTheme.safe : XTheme.textTertiary)
                    .frame(width: 20)
                Image(systemName: finding.severityIcon)
                    .font(.system(size: 15, weight: .medium))
                    .foregroundStyle(finding.severityColor)
                    .frame(width: 20)
                VStack(alignment: .leading, spacing: 3) {
                    HStack {
                        Text(finding.title)
                            .font(.system(size: 12, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                            .lineLimit(1)
                        Spacer()
                        if !finding.sizeFormatted.isEmpty {
                            Text(finding.sizeFormatted)
                                .font(.system(size: 11, weight: .medium))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                    }
                    FilePathDisplay(path: path, fontSize: 11, pathFontSize: 9, showIcon: false, maxPathLines: 1)
                    Text(finding.description)
                        .font(.system(size: 10))
                        .foregroundStyle(XTheme.textSecondary)
                        .lineLimit(2)
                }
            }
            .padding(.vertical, 5)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!isSafePath(path) || runner.isScanning)
        .help(isSafePath(path) ? "Select for review and move to Trash" : "Protected path")
    }

    private func isSafePath(_ path: String) -> Bool {
        let url = URL(fileURLWithPath: path).standardizedFileURL
        let home = FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL
        let temp = URL(fileURLWithPath: "/private/tmp").standardizedFileURL
        return url.path.hasPrefix(home.path + "/") || url.path.hasPrefix(temp.path + "/")
    }
}

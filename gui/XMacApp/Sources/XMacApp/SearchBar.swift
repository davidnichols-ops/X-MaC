import SwiftUI

struct SearchBar: View {
    @Binding var text: String

    var body: some View {
        HStack(spacing: 8) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(XTheme.textTertiary)
            TextField("Search findings...", text: $text)
                .textFieldStyle(.plain)
            if !text.isEmpty {
                Button {
                    text = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(XTheme.textTertiary)
                }
                .buttonStyle(.plain)
            }
        }
        .padding(10)
        .background(XTheme.bgTertiary)
        .clipShape(RoundedRectangle(cornerRadius: 8))
    }
}

import SwiftUI

// MARK: - Shared Scan UI Components
//
// Reusable components extracted from the old FullScanView for use across
// DiskView, MaintainView, and other scan-based views.

struct ScanProgressView: View {
    let phase: String

    var body: some View {
        VStack(spacing: 12) {
            ProgressView()
                .scaleEffect(0.8)
            Text(phase)
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textSecondary)
        }
        .frame(maxWidth: .infinity, minHeight: 120)
    }
}

struct ScanErrorView: View {
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "xmark.octagon.fill")
                .font(.system(size: 32))
                .foregroundStyle(XTheme.danger)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textSecondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, minHeight: 120)
    }
}

struct EmptyScanView: View {
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "checkmark.circle")
                .font(.system(size: 32))
                .foregroundStyle(XTheme.textTertiary)
            Text(message)
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textSecondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, minHeight: 120)
    }
}

struct StatBadge: View {
    let icon: String
    let label: String
    let value: String
    let color: Color

    var body: some View {
        VStack(spacing: 4) {
            Image(systemName: icon)
                .font(.system(size: 20))
                .foregroundStyle(color)
            Text(value)
                .font(.system(size: 20, weight: .bold, design: .rounded))
                .foregroundStyle(XTheme.textPrimary)
            Text(label)
                .font(.system(size: 11))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity)
    }
}

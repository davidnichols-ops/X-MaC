import SwiftUI

struct AutomationView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var isInstalled = false
    @State private var message: String?
    @AppStorage("autoScanEnabled") var autoScanEnabled: Bool = true
    @AppStorage("autoNotifyEnabled") var autoNotifyEnabled: Bool = true
    @AppStorage("quietHoursEnabled") var quietHoursEnabled: Bool = true
    @AppStorage("quietHoursStart") var quietHoursStart: Int = 22
    @AppStorage("quietHoursEnd") var quietHoursEnd: Int = 8

    private let scriptPath = Bundle.main.url(forResource: "install_launch_agent", withExtension: "sh")

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                header
                policyCard
                scheduleCard
                actionButtons
                if let message = message {
                    Text(message)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(message.contains("error") || message.contains("Failed") ? XTheme.danger : XTheme.safe)
                }
            }
            .padding(28)
        }
        .background(XTheme.voidGradient)
        .onAppear { checkStatus() }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Automation")
                .font(.system(size: 26, weight: .bold))
                .foregroundStyle(XTheme.textPrimary)
            Text("Schedule background scans and configure quiet hours. X-MaC never cleans automatically without your approval.")
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textSecondary)
                .frame(maxWidth: 640, alignment: .leading)
        }
    }

    private var policyCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Automation policy", icon: "shield.checkered")
                
                Toggle("Scan automatically", isOn: $autoScanEnabled)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textPrimary)
                
                Toggle("Notify when scan completes", isOn: $autoNotifyEnabled)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textPrimary)
                
                Toggle("Quiet hours (no notifications)", isOn: $quietHoursEnabled)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textPrimary)
                
                if quietHoursEnabled {
                    HStack {
                        Text("Quiet hours:")
                            .font(.system(size: 12))
                            .foregroundStyle(XTheme.textSecondary)
                        
                        Spacer()
                        
                        HStack(spacing: 8) {
                            Picker("Start", selection: $quietHoursStart) {
                                ForEach(0..<24) { hour in
                                    Text("\(hour, specifier: "%02d"):00").tag(hour)
                                }
                            }
                            .pickerStyle(.menu)
                            .frame(width: 80)
                            
                            Text("to")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                            
                            Picker("End", selection: $quietHoursEnd) {
                                ForEach(0..<24) { hour in
                                    Text("\(hour, specifier: "%02d"):00").tag(hour)
                                }
                            }
                            .pickerStyle(.menu)
                            .frame(width: 80)
                        }
                    }
                    .padding(.top, 4)
                }
                
                Divider().background(XTheme.cardBorder)
                
                AutomationPolicyRow(icon: "trash", text: "Move to Trash automatically: Never")
                AutomationPolicyRow(icon: "exclamationmark.octagon", text: "Permanent delete automatically: Never")
                
                Text("For safety, X-MaC never moves files to Trash or deletes them automatically. All cleanup actions require your explicit approval.")
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textTertiary)
                    .padding(.top, 4)
            }
        }
    }

    private var scheduleCard: some View {
        XCard {
            VStack(alignment: .leading, spacing: 12) {
                XSectionHeader(title: "Daily scan", icon: "calendar")
                
                Toggle("Run daily scan at 10:00 AM", isOn: $autoScanEnabled)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textPrimary)
                
                Text("A daily background scan runs at 10:00 AM using the macOS LaunchAgent facility. The scan writes JSON results to your local cache and does not modify files.")
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textSecondary)
                
                HStack(spacing: 8) {
                    Circle().fill(isInstalled ? XTheme.safe : XTheme.textTertiary).frame(width: 8, height: 8)
                    Text(isInstalled ? "LaunchAgent installed" : "LaunchAgent not installed")
                        .font(.system(size: 12, weight: .medium))
                        .foregroundStyle(isInstalled ? XTheme.safe : XTheme.textSecondary)
                }
            }
        }
    }

    private var actionButtons: some View {
        HStack(spacing: 12) {
            Button("Install LaunchAgent") { runScript("install") }
                .buttonStyle(.borderedProminent)
                .tint(XTheme.accent)
                .disabled(isInstalled)
            Button("Uninstall") { runScript("uninstall") }
                .buttonStyle(.bordered)
            Button("Check Status") { checkStatus() }
                .buttonStyle(.borderless)
        }
    }

    private func checkStatus() {
        Task {
            let (_, output) = await runner.runShellCommand(["/bin/launchctl", "list"], timeoutSecs: 10)
            await MainActor.run {
                isInstalled = output.contains("com.xmac.agent")
            }
        }
    }

    private func runScript(_ action: String) {
        guard let script = scriptPath else {
            message = "Failed: launch agent script not found in bundle."
            return
        }
        Task {
            let (status, output) = await runner.runShellCommand(["/bin/bash", script.path, action], timeoutSecs: 30)
            await MainActor.run {
                if status == 0 {
                    message = output.trimmingCharacters(in: .whitespacesAndNewlines)
                } else if output.hasPrefix("TIMEOUT") {
                    message = "Failed: command timed out"
                } else {
                    message = output.isEmpty ? "Failed with exit code \(status)" : output.trimmingCharacters(in: .whitespacesAndNewlines)
                }
                checkStatus()
            }
        }
    }
}

private struct AutomationPolicyRow: View {
    let icon: String
    let text: String

    var body: some View {
        HStack(spacing: 10) {
            Image(systemName: icon).foregroundStyle(XTheme.accent).xGlow(XTheme.accent, radius: 3).frame(width: 18)
            Text(text).font(.system(size: 12)).foregroundStyle(XTheme.textPrimary)
            Spacer()
        }
    }
}

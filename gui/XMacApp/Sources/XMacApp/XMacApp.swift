import SwiftUI

@main
struct XMacApp: App {
    @StateObject private var runner = XMacRunner()
    @StateObject private var settings = AppSettings()
    @StateObject private var profiles = ProfileStore()
    @StateObject private var crashReporter = CrashReporter()
    @StateObject private var adaptiveFixer = AdaptiveFixer.shared

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(runner)
                .environmentObject(settings)
                .environmentObject(profiles)
                .environmentObject(crashReporter)
                .environmentObject(adaptiveFixer)
                .preferredColorScheme(settings.appearance == "dark" ? .dark : settings.appearance == "light" ? .light : nil)
                .frame(minWidth: 1100, minHeight: 720)
                .onAppear {
                    runner.configure(settings: settings, profiles: profiles)
                    crashReporter.install()
                }
        }
        .windowStyle(.titleBar)
        .commands {
            CommandGroup(replacing: .newItem) {}
            CommandMenu("Scan") {
                Button("Full Scan") {
                    runner.startFullScan()
                }
                .keyboardShortcut("f", modifiers: [.command, .shift])

                Button("Neural Scan") {
                    runner.startNeuralScan()
                }
                .keyboardShortcut("n", modifiers: [.command, .shift])

                Divider()

                Button("Zen Mode — Preview") {
                    runner.openZen()
                    runner.runZen(execute: false)
                }
                .keyboardShortcut("z", modifiers: [.command, .shift])

                Button("AI Advisor — Analyze") {
                    runner.openAdvisor()
                    runner.runAdvisor()
                }
                .keyboardShortcut("a", modifiers: [.command, .shift])
            }
        }

        // Menu bar extra — quick access to X-MaC from the system menu bar
        MenuBarExtra {
            Button("Open X-MaC") {
                NSApplication.shared.activate(ignoringOtherApps: true)
                if NSApplication.shared.windows.isEmpty {
                    // Window might be closed — open a new one
                    NSApplication.shared.windows.first?.makeKeyAndOrderFront(nil)
                }
            }

            Divider()

            Button("Quick Clean") {
                runner.startCleanScan()
            }

            Button("Zen Mode — Preview") {
                runner.openZen()
                runner.runZen(execute: false)
            }

            Button("AI Advisor — Analyze") {
                runner.openAdvisor()
                runner.runAdvisor()
            }

            Divider()

            if runner.isScanning {
                Text("Scanning: \(runner.scanPhase)")
            } else {
                Text("System health: \(Int(runner.advisorHealthScore))/100")
            }

            Divider()

            Button("Quit X-MaC") {
                NSApplication.shared.terminate(nil)
            }
            .keyboardShortcut("q")
        } label: {
            Image(systemName: "cpu")
                .foregroundStyle(XTheme.accent)
        }
        .menuBarExtraStyle(.menu)
    }
}

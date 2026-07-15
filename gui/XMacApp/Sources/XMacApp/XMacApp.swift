import SwiftUI

@main
struct XMacApp: App {
    @StateObject private var runner = XMacRunner()
    @StateObject private var settings = AppSettings()
    @StateObject private var profiles = ProfileStore()
    @StateObject private var crashReporter = CrashReporter()
    @StateObject private var adaptiveFixer = AdaptiveFixer.shared
    @StateObject private var router = AppRouter()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(router)
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
                Button("Quick Scan") {
                    router.navigate(to: .improveQuickScan)
                    runner.startCleanScan()
                }
                .keyboardShortcut("f", modifiers: [.command, .shift])

                Button("Neural Scan") {
                    router.navigate(to: .improveNeuralScan)
                    runner.startNeuralScan()
                }
                .keyboardShortcut("n", modifiers: [.command, .shift])

                Divider()

                Button("Clean") {
                    router.navigate(to: .improveClean)
                    runner.startCleanScan()
                }
                .keyboardShortcut("c", modifiers: [.command, .shift])

                Button("What Changed?") {
                    router.navigate(to: .historyWhatChanged)
                }
                .keyboardShortcut("w", modifiers: [.command, .shift])

                Divider()

                Button("Zen Mode — Preview") {
                    router.navigate(to: .improveZenMode)
                    runner.runZen(execute: false)
                }
                .keyboardShortcut("z", modifiers: [.command, .shift])

                Button("AI Advisor — Analyze") {
                    router.navigate(to: .improveAIAdvisor)
                    runner.runAdvisor()
                }
                .keyboardShortcut("a", modifiers: [.command, .shift])
            }

            CommandMenu("Navigate") {
                Button("Home") { router.goHome() }
                    .keyboardShortcut("1", modifiers: [.command])

                Button("Explore — System") { router.navigate(to: .exploreSystem) }
                    .keyboardShortcut("2", modifiers: [.command])

                Button("Improve — Clean") { router.navigate(to: .improveClean) }
                    .keyboardShortcut("3", modifiers: [.command])

                Button("History — What Changed?") { router.navigate(to: .historyWhatChanged) }
                    .keyboardShortcut("4", modifiers: [.command])

                Button("Control — Settings") { router.navigate(to: .controlSettings) }
                    .keyboardShortcut("5", modifiers: [.command])

                Divider()

                Button("Refresh Current View") {
                    // Trigger re-render by toggling state
                    let dest = router.selectedDestination
                    router.navigate(to: .home)
                    DispatchQueue.main.asyncAfter(deadline: .now() + 0.01) {
                        router.navigate(to: dest)
                    }
                }
                .keyboardShortcut("r", modifiers: [.command])

                if runner.isScanning {
                    Divider()
                    Button("Cancel Current Operation") {
                        runner.stopScan()
                    }
                    .keyboardShortcut(".", modifiers: [.command])
                }
            }
        }

        // Menu bar extra — quick access to X-MaC from the system menu bar
        MenuBarExtra {
            Button("Open X-MaC") {
                NSApplication.shared.activate(ignoringOtherApps: true)
                if let window = NSApplication.shared.windows.first {
                    window.makeKeyAndOrderFront(nil)
                }
            }

            Divider()

            Button("Quick Scan") {
                router.navigate(to: .improveQuickScan)
                runner.startCleanScan()
            }

            Button("Clean") {
                router.navigate(to: .improveClean)
                runner.startCleanScan()
            }

            Button("Zen Mode — Preview") {
                router.navigate(to: .improveZenMode)
                runner.runZen(execute: false)
            }

            Button("AI Advisor — Analyze") {
                router.navigate(to: .improveAIAdvisor)
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
                .foregroundStyle(XTheme.color.accent)
        }
        .menuBarExtraStyle(.menu)
    }
}

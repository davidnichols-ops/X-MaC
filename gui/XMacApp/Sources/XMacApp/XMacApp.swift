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
            }
        }
    }
}

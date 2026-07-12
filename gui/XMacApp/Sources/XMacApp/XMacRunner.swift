import Foundation
import Combine

@MainActor
final class XMacRunner: ObservableObject {
    var historyStore = ScanHistoryStore()
    @Published var isScanning = false
    @Published var scanPhase: String = ""
    @Published var findings: [Finding] = []
    @Published var gnnScores: [GNNScore] = []
    @Published var gnnResponse: GNNResponse?
    @Published var purgePlan: PurgePlan?
    @Published var error: String?
    @Published var scanMode: ScanMode = .idle
    @Published var graphNodeCount: Int = 0
    @Published var scanDuration: Double = 0
    @Published var peakMemoryMB: Double = 0
    @Published var selectedPaths: Set<String> = []
    @Published var selectedGNNPaths: Set<String> = []
    @Published var cleanupResults: [CleanupResult] = []
    @Published var isCleaning = false
    @Published var cleanupMessage: String?
    @Published var binaryPathOverride: String? = nil
    @Published var scanProgress: Double = 0

    enum ScanMode: String {
        case dashboard, idle, full, clean, maintain, disk, neural, apps, settings, history, automation, ramBoost
    }

    private var appSettings: AppSettings?
    private var profileStore: ProfileStore?
    private var currentScanTask: Task<Void, Never>?
    private var binaryPathObserver: NSObjectProtocol?
    private var currentProcess: Process?

    func configure(settings: AppSettings, profiles: ProfileStore) {
        appSettings = settings
        profileStore = profiles

        if binaryPathObserver == nil {
            binaryPathObserver = NotificationCenter.default.addObserver(
                forName: .xmacOverrideBinaryPath,
                object: nil,
                queue: .main
            ) { [weak self] notification in
                Task { @MainActor [weak self] in
                    if let path = notification.userInfo?["path"] as? String {
                        self?.binaryPathOverride = path
                    }
                }
            }
        }
    }

    private let xmacPath: String = {
        let candidates = [
            "/Users/david/.local/bin/xmac",
            "/opt/homebrew/bin/xmac",
            "/usr/local/bin/xmac",
        ]
        for p in candidates {
            if FileManager.default.isExecutableFile(atPath: p) {
                return p
            }
        }
        return "xmac"
    }()

    func startFullScan() {
        guard !isScanning else { return }
        beginScan(mode: .full, phase: "Running full scan...", task: runFullScan)
    }

    func startCleanScan() {
        guard !isScanning else { return }
        beginScan(mode: .clean, phase: "Scanning for reclaimable space...", task: runCleanScan)
    }

    func startMaintainScan() {
        guard !isScanning else { return }
        beginScan(mode: .maintain, phase: "Running maintenance tasks...", task: runMaintainScan)
    }

    func startDiskScan() {
        guard !isScanning else { return }
        beginScan(mode: .disk, phase: "Analyzing disk usage...", task: runDiskScan)
    }

    func startNeuralScan() {
        guard !isScanning else { return }
        beginScan(mode: .neural, phase: "Extracting file system graph...", task: runNeuralScan)
    }

    private func beginScan(mode: ScanMode, phase: String, task: @escaping () async -> Void) {
        stopScan()
        clearResults()
        scanMode = mode
        isScanning = true
        scanPhase = phase
        scanProgress = 0.05
        currentScanTask = Task {
            await task()
        }
    }

    func stopScan() {
        currentProcess?.terminate()
        currentProcess = nil
        currentScanTask?.cancel()
        currentScanTask = nil
        if isScanning {
            isScanning = false
            scanPhase = ""
            scanProgress = 0
            error = error ?? "Scan cancelled"
        }
    }

    func openDashboard() {
        guard !isScanning else { return }
        scanMode = .dashboard
    }

    func openSettings() {
        guard !isScanning else { return }
        scanMode = .settings
    }

    func openHistory() {
        guard !isScanning else { return }
        scanMode = .history
    }

    func openApps() {
        guard !isScanning else { return }
        scanMode = .apps
    }

    func openAutomation() {
        guard !isScanning else { return }
        scanMode = .automation
    }

    func openRamBoost() {
        guard !isScanning else { return }
        scanMode = .ramBoost
    }

    func clearResults() {
        findings.removeAll()
        gnnScores.removeAll()
        gnnResponse = nil
        purgePlan = nil
        error = nil
        graphNodeCount = 0
        scanDuration = 0
        peakMemoryMB = 0
        selectedPaths.removeAll()
        selectedGNNPaths.removeAll()
        cleanupResults.removeAll()
        cleanupMessage = nil
    }

    func toggleSelection(path: String) {
        if selectedPaths.contains(path) {
            selectedPaths.remove(path)
        } else if isSafeCleanupPath(path) {
            selectedPaths.insert(path)
        }
    }

    func selectAllSafeFindings() {
        selectedPaths = Set(findings.compactMap { finding in
            guard let path = finding.target.value as String?, isSafeCleanupPath(path) else { return nil }
            return path
        })
    }

    func clearSelection() {
        selectedPaths.removeAll()
    }

    func trashSelected() {
        guard !isCleaning, !selectedPaths.isEmpty else { return }
        isCleaning = true
        cleanupResults.removeAll()
        cleanupMessage = nil
        let paths = Array(selectedPaths)
        Task { @MainActor in
            var results: [CleanupResult] = []
            for path in paths {
                let url = URL(fileURLWithPath: path)
                guard isSafeCleanupPath(path) else {
                    results.append(CleanupResult(path: path, success: false, message: "Blocked: protected system path"))
                    continue
                }
                do {
                    var resultingURL: NSURL?
                    try FileManager.default.trashItem(at: url, resultingItemURL: &resultingURL)
                    results.append(CleanupResult(path: path, success: true, message: "Moved to Trash"))
                } catch {
                    results.append(CleanupResult(path: path, success: false, message: error.localizedDescription))
                }
            }
            cleanupResults = results
            selectedPaths.removeAll()
            isCleaning = false
            let successful = results.filter(\.success).count
            cleanupMessage = "Moved \(successful) of \(results.count) selected items to Trash."
        }
    }

    // MARK: - GNN cleanup actions

    func toggleGNNSelection(path: String) {
        if selectedGNNPaths.contains(path) {
            selectedGNNPaths.remove(path)
        } else if isSafeCleanupPath(path) {
            selectedGNNPaths.insert(path)
        }
    }

    func selectGNNPaths(_ paths: [String]) {
        selectedGNNPaths.formUnion(paths.filter(isSafeCleanupPath))
    }

    func selectAllGNNSafePaths() {
        let safe = gnnScores
            .filter { $0.safety_score >= 0.7 }
            .map(\.path)
            .filter(isSafeCleanupPath)
        selectedGNNPaths = Set(safe)
    }

    func selectGNNPattern(examplePaths: [String]) {
        selectedGNNPaths.formUnion(examplePaths.filter(isSafeCleanupPath))
    }

    func clearGNNSelection() {
        selectedGNNPaths.removeAll()
    }

    func trashSelectedGNNPaths() {
        guard !isCleaning, !selectedGNNPaths.isEmpty else { return }
        isCleaning = true
        cleanupResults.removeAll()
        cleanupMessage = nil
        let paths = Array(selectedGNNPaths)
        Task { @MainActor in
            var results: [CleanupResult] = []
            for path in paths {
                let url = URL(fileURLWithPath: path)
                guard isSafeCleanupPath(path) else {
                    results.append(CleanupResult(path: path, success: false, message: "Blocked: protected system path"))
                    continue
                }
                do {
                    var resultingURL: NSURL?
                    try FileManager.default.trashItem(at: url, resultingItemURL: &resultingURL)
                    results.append(CleanupResult(path: path, success: true, message: "Moved to Trash"))
                } catch {
                    results.append(CleanupResult(path: path, success: false, message: error.localizedDescription))
                }
            }
            cleanupResults = results
            selectedGNNPaths.removeAll()
            isCleaning = false
            let successful = results.filter(\.success).count
            cleanupMessage = "Moved \(successful) of \(results.count) GNN-recommended items to Trash."
        }
    }

    func isSafeCleanupPath(_ path: String) -> Bool {
        let url = URL(fileURLWithPath: path).standardizedFileURL
        let home = FileManager.default.homeDirectoryForCurrentUser.standardizedFileURL
        let temporary = URL(fileURLWithPath: "/private/tmp").standardizedFileURL
        guard url.path != "/", url.path != home.path else { return false }
        return url.path.hasPrefix(home.path + "/") || url.path.hasPrefix(temporary.path + "/")
    }

    // MARK: - Scan implementations

    private func runFullScan() async {
        let start = Date()
        scanProgress = 0.1
        await runCommand(cleanCommand())
        guard !Task.isCancelled else { return }
        scanPhase = "Running maintenance..."
        scanProgress = 0.45
        await runCommand([xmacPath, "--format", "json", "maintain"])
        guard !Task.isCancelled else { return }
        scanPhase = "Analyzing disk usage..."
        scanProgress = 0.75
        await runCommand([xmacPath, "--format", "json", "disk", "--top", "30", "--min-size", "50M"])
        finishScan(start: start, mode: "full")
    }

    private func runCleanScan() async {
        let start = Date()
        scanProgress = 0.2
        await runCommand(cleanCommand())
        guard !Task.isCancelled else { return }
        finishScan(start: start, mode: "clean")
    }

    private func cleanCommand() -> [String] {
        let profile = profileStore?.selectedProfile
        let age = profile?.minimumAgeDays ?? appSettings?.minimumAgeDays ?? 30
        let size = profile?.minimumSizeMB ?? appSettings?.minimumSizeMB ?? 1
        let includeHidden = profile?.includeHidden ?? appSettings?.includeHidden ?? false
        let followSymlinks = profile?.followSymlinks ?? appSettings?.followSymlinks ?? false
        let exclusions = profile?.excludedPaths ?? appSettings?.exclusionList ?? []

        var command = [xmacPath, "--format", "json"]
        if includeHidden { command += ["--include-hidden"] }
        if followSymlinks { command += ["--follow-symlinks"] }
        for exclusion in exclusions {
            command += ["--exclude", exclusion]
        }
        command += ["clean", "--min-age", "\(age)d", "--min-size", "\(size)M", "--min-large-size", "100M"]
        return command
    }

    private func runMaintainScan() async {
        let start = Date()
        scanProgress = 0.2
        await runCommand([xmacPath, "--format", "json", "maintain"])
        guard !Task.isCancelled else { return }
        finishScan(start: start, mode: "maintain")
    }

    private func runDiskScan() async {
        let start = Date()
        scanProgress = 0.2
        await runCommand([xmacPath, "--format", "json", "disk", "--top", "40", "--min-size", "50M"])
        guard !Task.isCancelled else { return }
        finishScan(start: start, mode: "disk")
    }

    private func runNeuralScan() async {
        let start = Date()

        // Step 1: Extract graph to a temp file so we don't pay the cost of
        // decoding a 500KB nested graph through the Finding JSONValue enum.
        scanPhase = "Extracting file system graph..."
        scanProgress = 0.1
        let maxNodes = appSettings?.neuralMaxNodes ?? 600
        let graphDir = FileManager.default.temporaryDirectory.appendingPathComponent("xmac_gnn_\(UUID().uuidString)")
        var graphCommand = [xmacPath, "--format", "json"]
        if let settings = appSettings {
            graphCommand += settings.includeHidden ? ["--include-hidden"] : []
            for exclusion in settings.exclusionList {
                graphCommand += ["--exclude", exclusion]
            }
        }
        graphCommand += ["graph", "--max-depth", "8", "--max-nodes", "\(maxNodes)", "--output-graph", graphDir.path]
        let graphResult = await runCommandCollect(graphCommand)

        guard !Task.isCancelled else {
            try? FileManager.default.removeItem(at: graphDir)
            return
        }

        // Parse summary findings and discover the graph file path.
        scanProgress = 0.25
        var graphFileURL: URL?
        for line in graphResult.split(separator: "\n") {
            if let data = line.data(using: .utf8),
               let finding = try? JSONDecoder().decode(Finding.self, from: data) {
                if let nodes = finding.metadata["graph_nodes"]?.intValue {
                    graphNodeCount = nodes
                }
                findings.append(finding)
                if let outputPath = finding.remediation_hint?.replacingOccurrences(of: "cat ", with: ""),
                   outputPath.hasSuffix(".json"),
                   FileManager.default.fileExists(atPath: outputPath) {
                    graphFileURL = URL(fileURLWithPath: outputPath)
                }
            }
        }

        // Fall back to looking inside the output directory.
        if graphFileURL == nil {
            if let files = try? FileManager.default.contentsOfDirectory(at: graphDir, includingPropertiesForKeys: nil),
               let file = files.first(where: { $0.pathExtension == "json" }) {
                graphFileURL = file
            }
        }

        guard !Task.isCancelled else {
            try? FileManager.default.removeItem(at: graphDir)
            return
        }

        scanProgress = 0.35
        guard let graphFile = graphFileURL,
              let graphStr = try? String(contentsOf: graphFile, encoding: .utf8),
              !graphStr.isEmpty else {
            let message = "Failed to extract file system graph"
            error = message
            await CrashReporter.shared.record(
                error: NSError(domain: "XMacRunner", code: 1, userInfo: [NSLocalizedDescriptionKey: message]),
                context: "neuralScan"
            )
            try? FileManager.default.removeItem(at: graphDir)
            finishScan(start: start, mode: "neural")
            return
        }

        try? FileManager.default.removeItem(at: graphDir)

        // Step 2: Run CoreML inference
        scanPhase = "Running GNN inference on-device..."
        scanProgress = 0.45
        let manager = CoreMLGNNManager()

        // Animate progress during inference so the UI doesn't feel frozen.
        let progressTask = Task {
            let inferenceStart = Date()
            while !Task.isCancelled {
                let elapsed = Date().timeIntervalSince(inferenceStart)
                let fraction = min(elapsed / 5.0, 0.95)
                scanProgress = 0.45 + (0.40 * fraction)
                try? await Task.sleep(nanoseconds: 100_000_000)
            }
        }

        let response = await manager.predict(graphJSON: graphStr)
        progressTask.cancel()

        if Task.isCancelled { return }

        scanProgress = 0.9

        if let resp = response {
            gnnResponse = resp
            gnnScores = resp.scores
            purgePlan = resp.purge_plan
        } else {
            let message = "CoreML inference failed — check that XMacGNN.mlpackage is in the app bundle"
            error = message
            await CrashReporter.shared.record(
                error: NSError(domain: "XMacRunner", code: 2, userInfo: [NSLocalizedDescriptionKey: message]),
                context: "neuralScan"
            )
        }

        finishScan(start: start, mode: "neural")
    }

    private func finishScan(start: Date, mode: String = "unknown") {
        scanDuration = Date().timeIntervalSince(start)
        peakMemoryMB = getMemoryUsageMB()
        isScanning = false
        scanPhase = ""
        scanProgress = 1.0
        historyStore.add(mode: mode, findings: findings, duration: scanDuration)
    }

    // MARK: - Process execution

    private func runCommand(_ args: [String]) async {
        do {
            let (stdout, _) = try await runProcess(args)
            parseFindings(from: stdout)
        } catch let err {
            self.error = err.localizedDescription
            await CrashReporter.shared.record(
                error: err,
                context: "runCommand: \(args.first ?? "?")"
            )
        }
    }

    private func runCommandCollect(_ args: [String]) async -> String {
        do {
            let (stdout, _) = try await runProcess(args)
            return stdout
        } catch let err {
            self.error = err.localizedDescription
            return ""
        }
    }

    private func runProcess(_ args: [String]) async throws -> (String, String) {
        var args = args
        if let override = binaryPathOverride, !args.isEmpty, args[0].contains("xmac") {
            args[0] = override
        }
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<(String, String), any Error>) in
                let task = Process()
                self.currentProcess = task
                task.executableURL = URL(fileURLWithPath: args[0])
                task.arguments = Array(args.dropFirst())

                let stdoutPipe = Pipe()
                let stderrPipe = Pipe()
                task.standardOutput = stdoutPipe
                task.standardError = stderrPipe

                do {
                    try task.run()
                } catch {
                    self.currentProcess = nil
                    continuation.resume(throwing: error)
                    return
                }

                let group = DispatchGroup()

                var stdoutData = Data()
                var stderrData = Data()

                group.enter()
                DispatchQueue.global().async {
                    stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
                    group.leave()
                }

                group.enter()
                DispatchQueue.global().async {
                    stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
                    group.leave()
                }

                group.notify(queue: .global()) {
                    task.waitUntilExit()
                    self.currentProcess = nil
                    let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
                    let stderr = String(data: stderrData, encoding: .utf8) ?? ""
                    if task.terminationStatus != 0, task.terminationReason == .uncaughtSignal {
                        continuation.resume(throwing: NSError(domain: "XMacRunner", code: -1, userInfo: [NSLocalizedDescriptionKey: "Scan was cancelled"]))
                    } else {
                        continuation.resume(returning: (stdout, stderr))
                    }
                }
            }
        } onCancel: {
            Task { @MainActor in
                self.currentProcess?.terminate()
            }
        }
    }

    private func parseFindings(from output: String) {
        for line in output.split(separator: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard !trimmed.isEmpty,
                  trimmed.hasPrefix("{"),
                  let data = trimmed.data(using: .utf8) else { continue }
            if let finding = try? JSONDecoder().decode(Finding.self, from: data) {
                findings.append(finding)
            }
        }
    }

    private func getMemoryUsageMB() -> Double {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info>.size / MemoryLayout<integer_t>.size)
        let result = withUnsafeMutablePointer(to: &info) { ptr -> kern_return_t in
            ptr.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        if result == KERN_SUCCESS {
            return Double(info.phys_footprint) / 1_048_576.0
        }
        return 0
    }

    // MARK: - RAM Boost

    /// Run the `xmac ram-boost` command and parse the before/after memory snapshots.
    /// Returns (before, after, freedBytes, freedSwapBytes, processesKilled).
    func runRamBoost(
        purge: Bool,
        killTop: Int,
        killName: String?,
        force: Bool,
        minRssMB: Int,
        protectSystem: Bool
    ) async -> Result<(MemoryStatsSnapshot, MemoryStatsSnapshot?, UInt64, UInt64, Int), Error> {
        var args: [String] = [xmacPath, "--format", "json", "ram-boost"]
        if purge {
            args += ["--purge", "true"]
        } else {
            args += ["--purge", "false"]
        }
        if killTop > 0 {
            args += ["--kill-top", "\(killTop)"]
        }
        if let names = killName, !names.isEmpty {
            args += ["--kill-name", names]
        }
        if force {
            args += ["--force"]
        }
        args += ["--min-rss-mb", "\(minRssMB)"]
        if !protectSystem {
            args += ["--protect-system", "false"]
        }

        do {
            let (stdout, _) = try await runProcess(args)
            let (before, after, freed, freedSwap, killed) = parseRamBoostFindings(from: stdout)
            if before == nil {
                return .failure(NSError(domain: "XMacRunner", code: 10, userInfo: [
                    NSLocalizedDescriptionKey: "No memory data returned from ram-boost"
                ]))
            }
            return .success((before!, after, freed, freedSwap, killed))
        } catch {
            return .failure(error)
        }
    }

    /// Parse the JSON findings from `xmac ram-boost` and extract before/after snapshots.
    private func parseRamBoostFindings(from output: String) -> (MemoryStatsSnapshot?, MemoryStatsSnapshot?, UInt64, UInt64, Int) {
        var before: MemoryStatsSnapshot? = nil
        var after: MemoryStatsSnapshot? = nil
        var freed: UInt64 = 0
        var freedSwap: UInt64 = 0
        var killed: Int = 0

        for line in output.split(separator: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard trimmed.hasPrefix("{"),
                  let data = trimmed.data(using: .utf8),
                  let finding = try? JSONDecoder().decode(Finding.self, from: data) else { continue }

            // The metadata contains phase, used_bytes, available_bytes, etc.
            let phase = finding.metadata["phase"]?.stringValue ?? ""

            // Build a MemoryStatsSnapshot from the metadata fields
            let snapshot = MemoryStatsSnapshot(
                total_bytes: UInt64(finding.metadata["total_bytes"]?.intValue ?? 0),
                used_bytes: UInt64(finding.metadata["used_bytes"]?.intValue ?? 0),
                available_bytes: UInt64(finding.metadata["available_bytes"]?.intValue ?? 0),
                app_memory_bytes: UInt64(finding.metadata["app_memory_bytes"]?.intValue ?? 0),
                wired_bytes: UInt64(finding.metadata["wired_bytes"]?.intValue ?? 0),
                compressed_bytes: UInt64(finding.metadata["compressed_bytes"]?.intValue ?? 0),
                swap_used_bytes: UInt64(finding.metadata["swap_used_bytes"]?.intValue ?? 0),
                swap_total_bytes: UInt64(finding.metadata["swap_total_bytes"]?.intValue ?? 0),
                memory_pressure: finding.metadata["memory_pressure"]?.stringValue ?? "Nominal",
                top_consumers: []
            )

            if phase == "before" {
                before = snapshot
            } else if phase == "after" {
                after = snapshot
                freed = UInt64(finding.metadata["freed_bytes"]?.intValue ?? 0)
                freedSwap = UInt64(finding.metadata["freed_swap_bytes"]?.intValue ?? 0)
                killed = finding.metadata["processes_killed"]?.intValue ?? 0
            }
        }

        return (before, after, freed, freedSwap, killed)
    }
}

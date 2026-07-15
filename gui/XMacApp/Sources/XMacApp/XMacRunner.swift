import Foundation
import Combine

// MARK: - Stderr logging helper

private func logStderr(_ message: String) {
    message.withCString { cstr in
        fputs(cstr, stderr)
    }
}

// MARK: - Activity Log Entry

struct ActivityLogEntry: Identifiable, Hashable {
    let id = UUID()
    let timestamp: Date
    let operation: String
    let status: ActivityStatus
    let message: String
    let detail: String?
    let durationMs: Double?
}

enum ActivityStatus: String, Hashable {
    case started
    case success
    case warning
    case failed
}

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

    // Digital Twin data
    @Published var digitalTwin: DigitalTwin?
    @Published var twinLoading = false
    @Published var twinError: String?
    @Published var twinRecommendations: TwinRecommendations?
    @Published var twinReasoningResult: ReasoningResult?
    @Published var twinSandboxResult: SandboxResult?
    @Published var twinPredictions: [String] = []
    @Published var twinQueryResult: QueryResult?
    @Published var twinBenchmark: BenchmarkData?
    @Published var twinMonitoringPlan: MonitoringPlan?

    // What Changed? data
    @Published var whatChanged: WhatChangedReport?
    @Published var whatChangedLoading = false
    @Published var whatChangedError: String?

    // Generic scan results for unwired commands
    @Published var conflictFindings: [Finding] = []
    @Published var envmapFindings: [Finding] = []
    @Published var depthFindings: [Finding] = []
    @Published var quickFindings: [Finding] = []
    @Published var fullScanFindings: [Finding] = []
    @Published var purgeResult: String?
    @Published var configJson: String?
    @Published var configProfiles: [ConfigProfile] = []

    // Universal activity log — tracks all operations across the app
    @Published var activityLog: [ActivityLogEntry] = []
    @Published var lastActivity: ActivityLogEntry? = nil
    @Published var showActivityBanner: Bool = false

    enum ScanMode: String {
        // 10-tab architecture
        case overview, system, applications, filesystem, activity, optimization, intelligence, timeline, automation, assistant, settings
        // Legacy / sub-modes
        case dashboard, idle, full, clean, maintain, disk, neural, apps, history, ramBoost, zen, advisor
        case twin, twinHardware, twinSoftware, twinFilesystem, twinProcesses, twinMemory, twinEnergy, twinApps, twinReasoning
        case diagnostics, conflict, envmap, depth, quickScan, purge, configView
        case whatChanged
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

    // MARK: - Universal Activity Logger

    /// Log an activity. This is the single entry point for all operation tracking.
    func logActivity(_ operation: String, status: ActivityStatus, message: String, detail: String? = nil, durationMs: Double? = nil) {
        let entry = ActivityLogEntry(
            timestamp: Date(),
            operation: operation,
            status: status,
            message: message,
            detail: detail,
            durationMs: durationMs
        )
        activityLog.insert(entry, at: 0)
        if activityLog.count > 100 { activityLog.removeLast() }
        lastActivity = entry

        // Show banner for warnings and failures
        if status == .warning || status == .failed {
            showActivityBanner = true
        }

        // Always log to stderr so it's visible in Console.app
        let detailStr = detail != nil ? " — \(detail!)" : ""
        let durStr = durationMs != nil ? String(format: " [%.0fms]", durationMs!) : ""
        logStderr("[XMacRunner] \(status.rawValue.uppercased()) \(operation): \(message)\(detailStr)\(durStr)\n")
    }

    /// Track an async operation with automatic start/success/failure logging.
    func trackOperation<T>(_ operation: String, task: () async throws -> T) async -> Result<T, Error> {
        logActivity(operation, status: .started, message: "Starting...")
        let start = Date()
        do {
            let result = try await task()
            let duration = Date().timeIntervalSince(start) * 1000
            logActivity(operation, status: .success, message: "Completed", durationMs: duration)
            return .success(result)
        } catch {
            let duration = Date().timeIntervalSince(start) * 1000
            logActivity(operation, status: .failed, message: error.localizedDescription, durationMs: duration)
            return .failure(error)
        }
    }

    func dismissActivityBanner() {
        showActivityBanner = false
    }

    func clearActivityLog() {
        activityLog.removeAll()
        lastActivity = nil
    }

    let xmacPath: String = {
        // Look for the bundled binary first (inside the .app bundle), then
        // fall back to installed locations, then PATH.
        let bundleDir = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/xmac").path
        let candidates = [
            bundleDir,
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
        // Only stop if actually scanning — don't kill stale processes
        if isScanning {
            stopScan()
        }
        clearResults()
        scanMode = mode
        isScanning = true
        scanPhase = phase
        scanProgress = 0.05
        logActivity(mode.rawValue, status: .started, message: phase)
        currentScanTask = Task {
            await task()
        }
    }

    func stopScan() {
        if let proc = currentProcess, proc.isRunning {
            proc.terminate()
            logActivity("stopScan", status: .warning, message: "Process terminated")
        }
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

    func openZen() {
        guard !isScanning else { return }
        scanMode = .zen
    }

    func openAdvisor() {
        guard !isScanning else { return }
        scanMode = .advisor
    }

    // MARK: - Digital Twin Navigation

    func openTwin() {
        guard !isScanning else { return }
        scanMode = .twin
        if digitalTwin == nil && !twinLoading {
            collectTwin()
        }
    }

    func openTwinHardware() { scanMode = .twinHardware }
    func openTwinSoftware() { scanMode = .twinSoftware }
    func openTwinFilesystem() { scanMode = .twinFilesystem }
    func openTwinProcesses() { scanMode = .twinProcesses }
    func openTwinMemory() { scanMode = .twinMemory }
    func openTwinEnergy() { scanMode = .twinEnergy }
    func openTwinApps() { scanMode = .twinApps }
    func openTwinReasoning() { scanMode = .twinReasoning }

    func openDiagnostics() {
        guard !isScanning else { return }
        scanMode = .diagnostics
    }

    // 10-tab navigation
    func openOverview() { scanMode = .overview }
    func openSystem() { scanMode = .system }
    func openApplicationsTab() { scanMode = .applications }
    func openFilesystem() { scanMode = .filesystem }
    func openActivity() { scanMode = .activity }
    func openOptimization() { scanMode = .optimization }
    func openIntelligence() { scanMode = .intelligence }
    func openTimeline() { scanMode = .timeline }
    func openAssistant() { scanMode = .assistant }

    func openConflict() { scanMode = .conflict }
    func openEnvmap() { scanMode = .envmap }
    func openDepth() { scanMode = .depth }
    func openQuickScan() { scanMode = .quickScan }
    func openPurge() { scanMode = .purge }
    func openConfigView() { scanMode = .configView }

    // MARK: - Missing Command Scans

    func runConflictScan() {
        conflictFindings = []
        Task {
            await runCommand([xmacPath, "--format", "json", "conflict"])
            conflictFindings = findings
            scanMode = .conflict
        }
    }

    func runEnvmapScan() {
        envmapFindings = []
        Task {
            await runCommand([xmacPath, "--format", "json", "envmap"])
            envmapFindings = findings
            scanMode = .envmap
        }
    }

    func runDepthScan() {
        depthFindings = []
        Task {
            await runCommand([xmacPath, "--format", "json", "depth"])
            depthFindings = findings
            scanMode = .depth
        }
    }

    func runQuickScan() {
        quickFindings = []
        Task {
            await runCommand([xmacPath, "--format", "json", "quick"])
            quickFindings = findings
            scanMode = .quickScan
        }
    }

    func runFullSystemScan() {
        fullScanFindings = []
        Task {
            await runCommand([xmacPath, "--format", "json", "scan"])
            fullScanFindings = findings
            scanMode = .full
        }
    }

    func runConfigShow() {
        Task {
            let result = try? await runProcess([xmacPath, "--format", "json", "config", "show"])
            if let (stdout, _) = result {
                configJson = stdout
            }
        }
    }

    func runConfigProfiles() {
        Task {
            let result = try? await runProcess([xmacPath, "--format", "json", "config", "profiles"])
            if let (stdout, _) = result,
               let data = stdout.data(using: .utf8),
               let profiles = try? JSONDecoder().decode([ConfigProfile].self, from: data) {
                configProfiles = profiles
            }
        }
    }

    // MARK: - Digital Twin Data Loading

    func collectTwin() {
        twinLoading = true
        twinError = nil
        Task {
            let result = await trackOperation("Digital Twin Collect") { () async throws -> DigitalTwin in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "collect"]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin collect: no data")
                }
                return try JSONDecoder().decode(DigitalTwin.self, from: data)
            }
            switch result {
            case .success(let twin):
                self.digitalTwin = twin
                self.twinLoading = false
            case .failure(let err):
                self.twinError = err.localizedDescription
                self.twinLoading = false
            }
        }
    }

    func askTwin(question: String) {
        twinReasoningResult = nil
        Task {
            let result = await trackOperation("Twin Ask") { () async throws -> ReasoningResult in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "ask", "--question", question]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin ask: no data")
                }
                return try JSONDecoder().decode(ReasoningResult.self, from: data)
            }
            if case .success(let r) = result { self.twinReasoningResult = r }
        }
    }

    func predictTwin() {
        twinPredictions = []
        Task {
            let result = await trackOperation("Twin Predict") { () async throws -> [String] in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "predict"]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin predict: no data")
                }
                return try JSONDecoder().decode([String].self, from: data)
            }
            if case .success(let r) = result { self.twinPredictions = r }
        }
    }

    func simulateTwin(action: String) {
        twinSandboxResult = nil
        Task {
            let result = await trackOperation("Twin Simulate") { () async throws -> SandboxResult in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "simulate", "--simulate", action]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin simulate: no data")
                }
                return try JSONDecoder().decode(SandboxResult.self, from: data)
            }
            if case .success(let r) = result { self.twinSandboxResult = r }
        }
    }

    func recommendTwin() {
        twinRecommendations = nil
        Task {
            let result = await trackOperation("Twin Recommend") { () async throws -> TwinRecommendations in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "recommend"]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin recommend: no data")
                }
                return try JSONDecoder().decode(TwinRecommendations.self, from: data)
            }
            if case .success(let r) = result { self.twinRecommendations = r }
        }
    }

    func queryTwin(dimension: String) {
        twinQueryResult = nil
        Task {
            let result = await trackOperation("Twin Query") { () async throws -> QueryResult in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "query", "--query", dimension]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin query: no data")
                }
                return try JSONDecoder().decode(QueryResult.self, from: data)
            }
            if case .success(let r) = result { self.twinQueryResult = r }
        }
    }

    func benchmarkTwin() {
        twinBenchmark = nil
        Task {
            let result = await trackOperation("Twin Benchmark") { () async throws -> BenchmarkData in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "benchmark"]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin benchmark: no data")
                }
                return try JSONDecoder().decode(BenchmarkData.self, from: data)
            }
            if case .success(let r) = result { self.twinBenchmark = r }
        }
    }

    func monitorTwin() {
        twinMonitoringPlan = nil
        Task {
            let result = await trackOperation("Twin Monitor") { () async throws -> MonitoringPlan in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "monitor"]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin monitor: no data")
                }
                return try JSONDecoder().decode(MonitoringPlan.self, from: data)
            }
            if case .success(let r) = result { self.twinMonitoringPlan = r }
        }
    }

    // MARK: - What Changed?

    func queryWhatChanged(since: String) {
        whatChanged = nil
        whatChangedError = nil
        whatChangedLoading = true
        Task {
            let result = await trackOperation("What Changed?") { () async throws -> WhatChangedReport in
                let args = [self.xmacPath, "--format", "json", "twin", "--action", "what-changed", "--since", since]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("twin what-changed: no data")
                }
                return try JSONDecoder().decode(WhatChangedReport.self, from: data)
            }
            switch result {
            case .success(let report):
                self.whatChanged = report
                self.whatChangedLoading = false
            case .failure(let err):
                self.whatChangedError = err.localizedDescription
                self.whatChangedLoading = false
            }
        }
    }

    // MARK: - Safety Rules & Path Classification

    @Published var safetyRules: SafetyRulesResponse?
    @Published var safetyRulesLoading = false
    @Published var pathClassification: PathClassification?
    @Published var pathClassificationLoading = false

    func loadSafetyRules() {
        safetyRulesLoading = true
        Task {
            let result = await trackOperation("Load Safety Rules") { () async throws -> SafetyRulesResponse in
                let args = [self.xmacPath, "--format", "json", "safety", "--action", "list"]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("safety list: no data")
                }
                return try JSONDecoder().decode(SafetyRulesResponse.self, from: data)
            }
            switch result {
            case .success(let rules):
                self.safetyRules = rules
                self.safetyRulesLoading = false
            case .failure(let err):
                self.error = err.localizedDescription
                self.safetyRulesLoading = false
            }
        }
    }

    func classifyPath(_ path: String) {
        pathClassificationLoading = true
        Task {
            let result = await trackOperation("Classify Path") { () async throws -> PathClassification in
                let args = [self.xmacPath, "--format", "json", "safety", "--action", "classify", "--path", path]
                let (stdout, _) = try await self.runProcess(args)
                guard let data = stdout.data(using: .utf8) else {
                    throw XMacError.invalidOutput("safety classify: no data")
                }
                return try JSONDecoder().decode(PathClassification.self, from: data)
            }
            switch result {
            case .success(let classification):
                self.pathClassification = classification
                self.pathClassificationLoading = false
            case .failure(let err):
                self.error = err.localizedDescription
                self.pathClassificationLoading = false
            }
        }
    }

    // MARK: - Twin DB Management

    @Published var twinDbInitialized = false
    @Published var twinDbMessage: String?

    func initTwinDb() {
        Task {
            let result = await trackOperation("Init Twin DB") { () async throws -> String in
                let args = [self.xmacPath, "twin", "--action", "init-db"]
                let (_, stderr) = try await self.runProcess(args)
                return stderr
            }
            switch result {
            case .success(let msg):
                self.twinDbMessage = msg
                self.twinDbInitialized = true
            case .failure(let err):
                self.twinDbMessage = err.localizedDescription
            }
        }
    }

    func compactTwinDb() {
        Task {
            let result = await trackOperation("Compact Twin DB") { () async throws -> String in
                let args = [self.xmacPath, "twin", "--action", "compact"]
                let (_, stderr) = try await self.runProcess(args)
                return stderr
            }
            switch result {
            case .success(let msg):
                self.twinDbMessage = msg
            case .failure(let err):
                self.twinDbMessage = err.localizedDescription
            }
        }
    }

    // MARK: - Observer

    @Published var isObserving = false
    @Published var observeMessage: String?

    func runObservers(duration: String) {
        isObserving = true
        Task {
            let result = await trackOperation("Observe \(duration)") { () async throws -> String in
                let args = [self.xmacPath, "twin", "--action", "observe", "--duration", duration]
                let (_, stderr) = try await self.runProcess(args)
                return stderr
            }
            switch result {
            case .success(let msg):
                self.observeMessage = msg
                self.isObserving = false
            case .failure(let err):
                self.observeMessage = err.localizedDescription
                self.isObserving = false
            }
        }
    }

    // MARK: - Zen Mode

    @Published var zenResult: ZenResult?
    @Published var isZenRunning = false

    func runZen(execute: Bool) {
        guard !isZenRunning else { return }
        isZenRunning = true
        zenResult = nil
        logActivity("zen", status: .started, message: execute ? "Running Zen Mode..." : "Previewing Zen Mode...")
        Task { @MainActor in
            var args = [xmacPath, "--format", "json", "zen"]
            if execute { args += ["--execute"] }
            let result = await runCommandCollect(args)
            if let data = result.data(using: .utf8),
               let decoded = try? JSONDecoder().decode(ZenResult.self, from: data) {
                zenResult = decoded
                logActivity("zen", status: .success, message: "Zen Mode complete — health \(Int(decoded.health_before))→\(Int(decoded.health_after))")
            } else {
                error = "Failed to parse Zen Mode result"
                logActivity("zen", status: .failed, message: "Failed to parse result")
            }
            isZenRunning = false
        }
    }

    // MARK: - AI Advisor

    @Published var advisorRecommendations: [AdvisorRecommendation] = []
    @Published var advisorHealthScore: Double = 0
    @Published var advisorStatus: String = ""
    @Published var isAdvisorRunning = false

    func runAdvisor() {
        guard !isAdvisorRunning else { return }
        isAdvisorRunning = true
        advisorRecommendations = []
        logActivity("advisor", status: .started, message: "Analyzing system...")
        Task { @MainActor in
            let args = [xmacPath, "--format", "json", "advisor", "--advisor-format", "json", "--top", "20"]
            let result = await runCommandCollect(args)
            if let data = result.data(using: .utf8) {
                if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
                    if let score = json["health_score"] as? Double {
                        advisorHealthScore = score
                    }
                    if let status = json["status"] as? String {
                        advisorStatus = status
                    }
                    if let recs = json["recommendations"] as? [[String: Any]] {
                        advisorRecommendations = recs.compactMap { rec in
                            guard let data = try? JSONSerialization.data(withJSONObject: rec),
                                  let decoded = try? JSONDecoder().decode(AdvisorRecommendation.self, from: data) else {
                                return nil as AdvisorRecommendation?
                            }
                            return decoded
                        }
                    }
                }
                logActivity("advisor", status: .success, message: "Found \(advisorRecommendations.count) recommendations (health: \(Int(advisorHealthScore))/100)")
            } else {
                error = "Failed to parse advisor result"
                logActivity("advisor", status: .failed, message: "Failed to parse result")
            }
            isAdvisorRunning = false
        }
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
            // Block protected items from selection
            let isProtected = findings.contains { f in
                f.target.value == path && f.safety_rating?.lowercased() == "protected"
            }
            if !isProtected {
                selectedPaths.insert(path)
            }
        }
    }

    func selectAllSafeFindings() {
        selectedPaths = Set(findings.compactMap { finding in
            let path = finding.target.value
            guard isSafeCleanupPath(path) else { return nil }
            // Exclude protected items
            if let rating = finding.safety_rating?.lowercased(), rating == "protected" {
                return nil
            }
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
        logActivity(mode, status: .success, message: "Scan completed in \(String(format: "%.1f", scanDuration))s — \(findings.count) findings", durationMs: scanDuration * 1000)
    }

    // MARK: - Process execution

    private func runCommand(_ args: [String]) async {
        let cmdName = args.dropFirst().first ?? args.first ?? "?"
        do {
            let (stdout, stderr) = try await runProcess(args)
            parseFindings(from: stdout)
            if !stderr.isEmpty {
                logActivity(String(cmdName), status: .warning, message: "Process produced stderr", detail: stderr)
            }
        } catch let err {
            self.error = err.localizedDescription
            logActivity(String(cmdName), status: .failed, message: err.localizedDescription)
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

    func runProcess(_ args: [String]) async throws -> (String, String) {
        var args = args
        if let override = binaryPathOverride, !args.isEmpty, args[0].contains("xmac") {
            let bundlePath = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/xmac").path
            let allowedPaths: Set<String> = [
                bundlePath,
                "/opt/homebrew/bin/xmac",
                "/usr/local/bin/xmac",
            ]
            if allowedPaths.contains(override) {
                args[0] = override
            } else {
                logStderr("[XMacRunner] runProcess: ignoring binaryPathOverride '\(override)' — not in whitelist\n")
            }
        }

        let cmdDesc = "\(args[0]) \(args.dropFirst().joined(separator: " "))"
        logStderr("[XMacRunner] runProcess: \(cmdDesc)\n")

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
                    logStderr("[XMacRunner] runProcess: started pid=\(task.processIdentifier)\n")
                } catch {
                    self.currentProcess = nil
                    logStderr("[XMacRunner] runProcess: FAILED to start — \(error.localizedDescription)\n")
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
                    let status = task.terminationStatus
                    let reason = task.terminationReason
                    logStderr("[XMacRunner] runProcess: exited status=\(status) reason=\(reason.rawValue) stdout=\(stdout.count)b stderr=\(stderr.count)b\n")

                    if status != 0 && reason == .uncaughtSignal {
                        continuation.resume(throwing: NSError(domain: "XMacRunner", code: -1, userInfo: [NSLocalizedDescriptionKey: "Scan was cancelled"]))
                    } else if status != 0 && stdout.isEmpty {
                        // Process failed with no stdout — likely an error
                        let errMsg = stderr.isEmpty ? "Process exited with status \(status)" : stderr
                        continuation.resume(throwing: NSError(domain: "XMacRunner", code: Int(status), userInfo: [NSLocalizedDescriptionKey: errMsg]))
                    } else {
                        continuation.resume(returning: (stdout, stderr))
                    }
                }
            }
        } onCancel: {
            Task { @MainActor in
                self.currentProcess?.terminate()
                logStderr("[XMacRunner] runProcess: cancelled by task\n")
            }
        }
    }

    // MARK: - Privileged Command Execution (sudo via AppleScript)

    /// Run a shell command with administrator privileges.
    /// Shows the native macOS password dialog via AppleScript.
    /// Returns (success, output) — output includes stdout+stderr.
    func runPrivileged(_ command: String) async -> (success: Bool, output: String) {
        logStderr("[XMacRunner] runPrivileged: \(command)")

        // Escape backslashes, double quotes, newlines, carriage returns, and tabs for AppleScript.
        // NOTE: This is still string interpolation into an AppleScript string literal, which is
        // fragile against crafted input. Constructing an NSAppleEventDescriptor and passing the
        // command via a structured Apple event would be more secure and avoid escaping entirely.
        let escaped = command
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
            .replacingOccurrences(of: "\r", with: "\\r")
            .replacingOccurrences(of: "\t", with: "\\t")

        let script = "do shell script \"\(escaped)\" with administrator privileges"

        let task = Process()
        task.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        task.arguments = ["-e", script]

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        task.standardOutput = stdoutPipe
        task.standardError = stderrPipe

        do {
            try task.run()
        } catch {
            logStderr("[XMacRunner] runPrivileged: FAILED to start osascript — \(error.localizedDescription)")
            return (false, error.localizedDescription)
        }

        // Store so it can be cancelled
        currentProcess = task

        let stdoutData = stdoutPipe.fileHandleForReading.readDataToEndOfFile()
        let stderrData = stderrPipe.fileHandleForReading.readDataToEndOfFile()
        task.waitUntilExit()
        currentProcess = nil

        let stdout = String(data: stdoutData, encoding: .utf8) ?? ""
        let stderr = String(data: stderrData, encoding: .utf8) ?? ""
        let status = task.terminationStatus

        logStderr("[XMacRunner] runPrivileged: exited status=\(status) stdout=\(stdout.count)b stderr=\(stderr.count)b")

        if status == 0 {
            return (true, stdout)
        } else {
            // Common error: user cancelled the password dialog
            // osascript returns "User canceled" or similar
            let msg = stderr.isEmpty ? "Command failed with status \(status)" : stderr
            return (false, msg)
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

    /// Run the RAM boost: before snapshot → purge (with admin privileges) → kill processes → after snapshot.
    /// Returns (before, after, freedBytes, freedSwapBytes, processesKilled, purgeSuccess, purgeMessage).
    func runRamBoost(
        purge: Bool,
        killTop: Int,
        killName: String?,
        force: Bool,
        minRssMB: Int,
        protectSystem: Bool
    ) async -> Result<(MemoryStatsSnapshot, MemoryStatsSnapshot?, UInt64, UInt64, Int, Bool, String), Error> {

        let start = Date()
        logActivity("ram-boost", status: .started, message: purge ? "Boosting memory (purge + processes)..." : "Reading memory stats...")

        // Step 1: Before snapshot (always report-only to get clean before state)
        do {
            let beforeArgs: [String] = [xmacPath, "--format", "json", "ram-boost", "--report-only"]
            let (beforeOut, beforeErr) = try await runProcess(beforeArgs)
            if !beforeErr.isEmpty {
                logActivity("ram-boost", status: .warning, message: "Before snapshot stderr", detail: beforeErr)
            }

            let (beforeSnap, _, _, _, _) = parseRamBoostFindings(from: beforeOut)
            guard let before = beforeSnap else {
                let msg = "No memory data returned from ram-boost"
                logActivity("ram-boost", status: .failed, message: msg, detail: beforeOut.isEmpty ? "Empty output" : "Output: \(beforeOut.prefix(200))")
                return .failure(NSError(domain: "XMacRunner", code: 10, userInfo: [NSLocalizedDescriptionKey: msg]))
            }

            // If this is report-only, return now
            if !purge && killTop == 0 && (killName == nil || killName!.isEmpty) {
                let duration = Date().timeIntervalSince(start) * 1000
                logActivity("ram-boost", status: .success, message: "Memory report generated", durationMs: duration)
                return .success((before, nil, 0, 0, 0, true, "Report only"))
            }

            // Step 2: Purge with admin privileges if requested
            var purgeSuccess = false
            var purgeMessage = ""
            if purge {
                logActivity("ram-boost", status: .started, message: "Requesting admin privileges to purge memory...")
                let (ok, output) = await runPrivileged("purge")
                purgeSuccess = ok
                if ok {
                    purgeMessage = "Inactive memory purged successfully"
                    logActivity("ram-boost purge", status: .success, message: purgeMessage)
                } else {
                    purgeMessage = output.contains("User canceled") || output.contains("canceled")
                        ? "Purge cancelled — password dialog dismissed"
                        : "Purge failed: \(output)"
                    logActivity("ram-boost purge", status: .warning, message: purgeMessage)
                    // Continue anyway — we still take the after snapshot
                }
            }

            // Step 3: Kill processes if requested (via xmac, no sudo needed for user processes)
            var killed = 0
            if killTop > 0 || (killName != nil && !killName!.isEmpty) {
                var killArgs: [String] = [xmacPath, "--format", "json", "ram-boost", "--purge", "false"]
                if killTop > 0 {
                    killArgs += ["--kill-top", "\(killTop)"]
                }
                if let names = killName, !names.isEmpty {
                    killArgs += ["--kill-name", names]
                }
                if force {
                    killArgs += ["--force"]
                }
                killArgs += ["--min-rss-mb", "\(minRssMB)"]
                if !protectSystem {
                    killArgs += ["--protect-system", "false"]
                }

                do {
                    let (killOut, _) = try await runProcess(killArgs)
                    let (_, _, _, _, killCount) = parseRamBoostFindings(from: killOut)
                    killed = killCount
                } catch {
                    logActivity("ram-boost kill", status: .warning, message: "Process kill step failed: \(error.localizedDescription)")
                }
            }

            // Step 4: After snapshot (report-only, to get clean after state)
            let afterArgs: [String] = [xmacPath, "--format", "json", "ram-boost", "--report-only"]
            let (afterOut, _) = try await runProcess(afterArgs)
            let (_, afterSnap, _, _, _) = parseRamBoostFindings(from: afterOut)

            // Calculate freed bytes
            let freed: UInt64 = afterSnap != nil && before.used_bytes > afterSnap!.used_bytes
                ? before.used_bytes - afterSnap!.used_bytes
                : 0
            let freedSwap: UInt64 = afterSnap != nil && before.swap_used_bytes > afterSnap!.swap_used_bytes
                ? before.swap_used_bytes - afterSnap!.swap_used_bytes
                : 0

            let duration = Date().timeIntervalSince(start) * 1000
            if freed > 0 {
                logActivity("ram-boost", status: .success, message: "Freed \(formatBytes(freed)) RAM, \(formatBytes(freedSwap)) swap, \(killed) processes killed", durationMs: duration)
            } else if purgeSuccess {
                logActivity("ram-boost", status: .success, message: "Purge completed. \(killed) processes killed. Memory may have been reallocated already.", durationMs: duration)
            } else {
                logActivity("ram-boost", status: .warning, message: purgeMessage, durationMs: duration)
            }

            return .success((before, afterSnap, freed, freedSwap, killed, purgeSuccess, purgeMessage))
        } catch {
            let duration = Date().timeIntervalSince(start) * 1000
            logActivity("ram-boost", status: .failed, message: error.localizedDescription, durationMs: duration)
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

            // Parse top_consumers from metadata array
            var consumers: [ProcessMemoryEntry] = []
            if case .array(let arr) = finding.metadata["top_consumers"] ?? .null {
                for item in arr {
                    if case .object(let obj) = item {
                        consumers.append(ProcessMemoryEntry(
                            pid: UInt32(obj["pid"]?.intValue ?? 0),
                            name: obj["name"]?.stringValue ?? "unknown",
                            rss_bytes: UInt64(obj["rss_bytes"]?.intValue ?? 0),
                            percent: obj["percent"]?.doubleValue ?? 0.0
                        ))
                    }
                }
            }

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
                top_consumers: consumers
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

    // MARK: - Memory Optimize (GNN)

    @Published var optimizeResult: OptimizeSnapshot? = nil
    @Published var isOptimizing = false

    /// Run `xmac optimize --observe true --format json` and parse the output.
    /// Returns the snapshot with graph JSON, system telemetry, and process list.
    func runOptimize(topN: Int = 15) async {
        await MainActor.run {
            isOptimizing = true
            optimizeResult = nil
        }

        logActivity("optimize", status: .started, message: "Collecting memory telemetry...")

        do {
            let args: [String] = [xmacPath, "--format", "json", "optimize", "--observe", "true",
                                  "--top-n", "\(topN)"]
            let (output, _) = try await runProcess(args)

            let snapshot = parseOptimizeOutput(output)

            await MainActor.run {
                self.optimizeResult = snapshot
                self.isOptimizing = false
            }

            logActivity("optimize", status: .success,
                        message: "Memory graph: \(snapshot.processes.count) processes, pressure: \(snapshot.pressureLabel)")

            // Run GNN inference if we have a graph
            if let graphJSON = snapshot.graphJSON {
                await runGNNPrediction(graphJSON: graphJSON)
            }

        } catch {
            await MainActor.run {
                self.isOptimizing = false
            }
            logActivity("optimize", status: .failed, message: error.localizedDescription)
        }
    }

    /// Run the CoreML GNN model on the graph JSON and update the snapshot with predictions.
    private func runGNNPrediction(graphJSON: String) async {
        let manager = MemoryGNNManager()
        let result = await manager.predict(graphJSON: graphJSON)

        if let result = result {
            await MainActor.run {
                self.optimizeResult?.gnnResult = result
            }
            logActivity("optimize-gnn", status: .success,
                        message: "GNN: \(result.predictions.count) predictions, pressure: \(result.systemPressure.level), fallback: \(result.usingFallback)")
        } else {
            logActivity("optimize-gnn", status: .warning, message: "GNN prediction returned nil")
        }
    }

    /// Parse the JSON output from `xmac optimize --format json`.
    private func parseOptimizeOutput(_ output: String) -> OptimizeSnapshot {
        var graphJSON: String? = nil
        var processes: [OptimizeProcess] = []
        var pressureLabel: String = "Nominal"
        var predictedPressure: Double = 0
        var utilizationPct: Double = 0
        var freeMB: Double = 0
        var compressedMB: Double = 0
        var swapMB: Double = 0
        var totalMB: Double = 0
        var usedMB: Double = 0
        var nodeCount: Int = 0
        var edgeCount: Int = 0

        for line in output.split(separator: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            guard trimmed.hasPrefix("{"),
                  let data = trimmed.data(using: .utf8),
                  let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { continue }

            let category = obj["category"] as? String ?? ""
            let desc = obj["description"] as? String ?? ""
            let metadata = obj["metadata"] as? [String: Any] ?? [:]

            if category == "system_info" {
                // Extract the graph as JSON string for CoreML
                if let graph = metadata["graph"],
                   let graphData = try? JSONSerialization.data(withJSONObject: graph),
                   let graphStr = String(data: graphData, encoding: .utf8) {
                    graphJSON = graphStr
                }

                // Parse description: "Collected memory graph: 55 nodes, 68 edges. Pressure: Nominal, utilization: 60.8%, free: 447.5 MB, compressed: 1.0 GB, swap: 0.0 B"
                let parts = desc.split(separator: ",")
                for part in parts {
                    let p = part.trimmingCharacters(in: .whitespaces)
                    if p.contains("nodes") {
                        // "Collected memory graph: 55 nodes" → extract number before "nodes"
                        let words = p.split(separator: " ")
                        if let n = words.first(where: { Int($0) != nil }).flatMap({ Int($0) }) { nodeCount = n }
                    }
                    if p.contains("edges") {
                        let words = p.split(separator: " ")
                        if let e = words.first(where: { Int($0) != nil }).flatMap({ Int($0) }) { edgeCount = e }
                    }
                    if p.hasPrefix("Pressure:") {
                        let val = p.replacingOccurrences(of: "Pressure:", with: "").trimmingCharacters(in: .whitespaces)
                        pressureLabel = val
                    }
                    if p.hasPrefix("utilization:") {
                        let val = p.replacingOccurrences(of: "utilization:", with: "").replacingOccurrences(of: "%", with: "").trimmingCharacters(in: .whitespaces)
                        utilizationPct = Double(val) ?? 0
                    }
                    if p.hasPrefix("free:") {
                        let val = p.replacingOccurrences(of: "free:", with: "").trimmingCharacters(in: .whitespaces)
                        let words = val.split(separator: " ")
                        if words.count >= 2 {
                            freeMB = Double(words[0]) ?? 0
                            if words[1] == "GB" { freeMB *= 1024 }
                        }
                    }
                    if p.hasPrefix("compressed:") {
                        let val = p.replacingOccurrences(of: "compressed:", with: "").trimmingCharacters(in: .whitespaces)
                        let words = val.split(separator: " ")
                        if words.count >= 2 {
                            compressedMB = Double(words[0]) ?? 0
                            if words[1] == "GB" { compressedMB *= 1024 }
                        }
                    }
                    if p.hasPrefix("swap:") {
                        let val = p.replacingOccurrences(of: "swap:", with: "").trimmingCharacters(in: .whitespaces)
                        let words = val.split(separator: " ")
                        if words.count >= 2 {
                            swapMB = Double(words[0]) ?? 0
                            if words[1] == "GB" { swapMB *= 1024 }
                        }
                    }
                }

                if let pp = metadata["predicted_pressure_60s"] as? Double {
                    predictedPressure = pp
                } else if let pp = metadata["predicted_pressure_60s"] as? Int {
                    predictedPressure = Double(pp)
                }
            }

            if let pt = metadata["process_telemetry"] as? [String: Any] {
                let name = pt["name"] as? String ?? "unknown"
                let pid = pt["pid"] as? Int ?? 0
                let rss = pt["resident_size"] as? Int ?? 0
                let virtual = pt["virtual_size"] as? Int ?? 0
                let threads = pt["thread_count"] as? Int ?? 0
                let isSystem = pt["is_system"] as? Bool ?? false
                let purgeable = pt["purgeable_volatile"] as? Int ?? 0
                let compressed = pt["compressed_bytes"] as? Int ?? 0
                let pageFaults = pt["page_faults"] as? Int ?? 0
                let physFootprint = pt["phys_footprint"] as? Int ?? 0
                let severity = obj["severity"] as? String ?? "info"

                processes.append(OptimizeProcess(
                    pid: pid,
                    name: name,
                    rssBytes: UInt64(rss),
                    virtualBytes: UInt64(virtual),
                    threadCount: threads,
                    isSystem: isSystem,
                    purgeableBytes: UInt64(purgeable),
                    compressedBytes: UInt64(compressed),
                    pageFaults: UInt64(pageFaults),
                    physFootprint: UInt64(physFootprint),
                    severity: severity
                ))
            }
        }

        // Calculate total/used from utilization
        // On a Mac with 16GB: utilization 60% → used = 9.6GB, total = 16GB
        // We can estimate total from the hardware node in the graph, but for now
        // use the process list sum as an approximation of used
        let totalRSS = processes.reduce(0.0) { $0 + Double($1.rssBytes) / 1_048_576 }
        usedMB = totalRSS > 0 ? totalRSS : (utilizationPct / 100.0) * 16384
        totalMB = utilizationPct > 0 ? (usedMB / (utilizationPct / 100.0)) : 16384

        return OptimizeSnapshot(
            graphJSON: graphJSON,
            processes: processes,
            pressureLabel: pressureLabel,
            predictedPressure: predictedPressure,
            utilizationPct: utilizationPct,
            freeMB: freeMB,
            compressedMB: compressedMB,
            swapMB: swapMB,
            totalMB: totalMB,
            usedMB: usedMB,
            nodeCount: nodeCount,
            edgeCount: edgeCount,
            gnnResult: nil
        )
    }
}

// MARK: - Optimize Models

struct OptimizeSnapshot: Identifiable {
    let id = UUID()
    let graphJSON: String?
    var processes: [OptimizeProcess]
    let pressureLabel: String
    let predictedPressure: Double
    let utilizationPct: Double
    let freeMB: Double
    let compressedMB: Double
    let swapMB: Double
    let totalMB: Double
    let usedMB: Double
    let nodeCount: Int
    let edgeCount: Int
    var gnnResult: MemoryGNNManager.OptimizationResult?
}

struct OptimizeProcess: Identifiable, Hashable {
    let pid: Int
    let name: String
    let rssBytes: UInt64
    let virtualBytes: UInt64
    let threadCount: Int
    let isSystem: Bool
    let purgeableBytes: UInt64
    let compressedBytes: UInt64
    let pageFaults: UInt64
    let physFootprint: UInt64
    let severity: String

    var id: Int { pid }
    var rssMB: Double { Double(rssBytes) / 1_048_576 }
    var virtualGB: Double { Double(virtualBytes) / 1_073_741_824 }

    /// Find the GNN prediction for this process (if available).
    /// Matches by name since graph nodes use labels, not real PIDs.
    func gnnPrediction(from result: MemoryGNNManager.OptimizationResult?) -> MemoryGNNManager.MemoryPrediction? {
        guard let result = result else { return nil }
        // Try exact name match first
        if let pred = result.predictions.first(where: { $0.processName == name }) {
            return pred
        }
        // Try case-insensitive match
        if let pred = result.predictions.first(where: { $0.processName.lowercased() == name.lowercased() }) {
            return pred
        }
        return nil
    }
}

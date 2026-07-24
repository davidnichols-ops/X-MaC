import Foundation
import CoreML

/// stderr logging for debugging
private func gnnLog(_ msg: String) {
    msg.withCString { cstr in
        fputs(cstr, stderr)
        fputs("\n", stderr)
    }
}

/// Manages the Memory Optimizer CoreML model (XMacMemoryGNN).
///
/// The model takes per-node features (24 dims, padded) and outputs:
///   - action_logits (6): no_action, pressure_relief, suggest_purge, deprioritize, suspend, terminate
///   - risk (1): 0.0–1.0
///   - growth (1): normalized predicted RSS growth
///   - pressure (3): normal, warn, critical (softmax)
///
/// Total output: 11 values per node.
///
/// Thread safety: model state is protected by NSLock for concurrent access.
final class MemoryGNNManager {
    private let stateLock = NSLock()
    private var _model: MLModel?

    private var model: MLModel? {
        stateLock.lock()
        defer { stateLock.unlock() }
        return _model
    }

    // Output layout: [action(6), risk(1), growth(1), pressure(3)] = 11
    static let featureCount = 24
    static let outputCount = 11
    static let actionCount = 6
    static let pressureCount = 3

    static let actionNames = [
        "no_action", "pressure_relief", "suggest_purge",
        "deprioritize", "suspend", "terminate",
    ]
    static let pressureNames = ["normal", "warn", "critical"]

    init() {}

    // MARK: - Model Loading

    private func loadModel() -> MLModel? {
        stateLock.lock()
        if let m = _model {
            stateLock.unlock()
            return m
        }
        stateLock.unlock()

        let cacheDir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("XMacMemoryGNN", isDirectory: true)
        let compiledCacheURL = cacheDir.appendingPathComponent("XMacMemoryGNN.mlmodelc")

        if FileManager.default.fileExists(atPath: compiledCacheURL.path) {
            do {
                let m = try MLModel(contentsOf: compiledCacheURL)
                gnnLog("[MemoryGNN] Compiled model loaded from cache.")
                stateLock.lock()
                _model = m
                stateLock.unlock()
                return m
            } catch {
                gnnLog("[MemoryGNN] Cached model failed, recompiling...")
            }
        }

        guard let bundleURL = Bundle.main.url(forResource: "XMacMemoryGNN", withExtension: "mlpackage") else {
            gnnLog("[MemoryGNN] Model not found in bundle.")
            return nil
        }

        gnnLog("[MemoryGNN] Compiling .mlpackage...")
        do {
            let compiledURL = try MLModel.compileModel(at: bundleURL)
            try? FileManager.default.createDirectory(at: cacheDir, withIntermediateDirectories: true)
            try? FileManager.default.removeItem(at: compiledCacheURL)
            try? FileManager.default.copyItem(at: compiledURL, to: compiledCacheURL)
            let m = try MLModel(contentsOf: compiledCacheURL)
            gnnLog("[MemoryGNN] Model compiled and loaded.")
            stateLock.lock()
            _model = m
            stateLock.unlock()
            return m
        } catch {
            gnnLog("[MemoryGNN] Compilation failed: \(error)")
            return nil
        }
    }

    // MARK: - Prediction

    struct MemoryPrediction {
        let processName: String
        let pid: Int
        let action: String
        let actionIndex: Int
        let actionConfidence: Double
        let risk: Double
        let growth: Double
        let pressure: String
        let pressureConfidence: Double
    }

    struct SystemPressurePrediction {
        let level: String      // normal, warn, critical
        let confidence: Double
    }

    struct OptimizationResult {
        let predictions: [MemoryPrediction]
        let systemPressure: SystemPressurePrediction
        let usingFallback: Bool
    }

    /// Run inference on a memory graph JSON (from the Rust `optimize --observe --format json` output).
    func predict(graphJSON: String) async -> OptimizationResult? {
        guard let graphData = graphJSON.data(using: .utf8),
              let graph = try? JSONSerialization.jsonObject(with: graphData) as? [String: Any],
              let nodes = graph["nodes"] as? [[String: Any]] else {
            gnnLog("[MemoryGNN] Failed to parse graph JSON")
            return nil
        }

        guard !nodes.isEmpty else {
            gnnLog("[MemoryGNN] Empty graph — no nodes")
            return OptimizationResult(
                predictions: [],
                systemPressure: SystemPressurePrediction(level: "normal", confidence: 1.0),
                usingFallback: false
            )
        }

        let nodeCount = min(nodes.count, 600)
        let cappedNodes = Array(nodes.prefix(nodeCount))

        // Pad all node features to featureCount (24). Different node types
        // (process, hardware, swap, compressor) may have different raw feature
        // counts — the model was trained on padded 24-dim vectors.
        let paddedNodes: [[String: Any]] = cappedNodes.map { node in
            guard let features = node["features"] as? [Any] else { return node }
            if features.count >= Self.featureCount { return node }
            // Pad with zeros
            var padded = features
            while padded.count < Self.featureCount {
                padded.append(0.0)
            }
            var newNode = node
            newNode["features"] = padded
            return newNode
        }

        // Validate feature dimensions
        guard paddedNodes.allSatisfy({ node in
            guard let features = node["features"] as? [Any] else {
                gnnLog("[MemoryGNN] Node missing features: \(node["node_type"] ?? "?")")
                return false
            }
            return features.count == Self.featureCount
        }) else {
            gnnLog("[MemoryGNN] Invalid node feature dimensions after padding — using fallback")
            return fallbackResult(nodes: paddedNodes)
        }

        guard let loadedModel = loadModel() else {
            gnnLog("[MemoryGNN] No model available — using fallback")
            return fallbackResult(nodes: paddedNodes)
        }

        // Create MLMultiArray input
        guard let nodeFeatures = try? MLMultiArray(
            shape: [NSNumber(value: nodeCount), NSNumber(value: Self.featureCount)],
            dataType: .float32
        ) else {
            gnnLog("[MemoryGNN] Failed to create MLMultiArray input")
            return fallbackResult(nodes: paddedNodes)
        }

        for (i, node) in paddedNodes.enumerated() {
            guard let features = node["features"] as? [Any] else { continue }
            for (j, feature) in features.enumerated() {
                let val: Double
                if let n = feature as? NSNumber { val = n.doubleValue }
                else if let d = feature as? Double { val = d }
                else if let n = feature as? Int { val = Double(n) }
                else { continue }
                nodeFeatures[[NSNumber(value: i), NSNumber(value: j)]] = NSNumber(value: Float(val))
            }
        }

        let inputDict: [String: Any] = ["node_features": nodeFeatures]

        do {
            let features = try MLDictionaryFeatureProvider(dictionary: inputDict)
            let output = try await withTimeout(seconds: 5) {
                try await loadedModel.prediction(from: features)
            }

            guard let predictions = output.featureValue(for: "predictions")?.multiArrayValue,
                  predictions.shape.count == 2,
                  predictions.shape[0].intValue == nodeCount,
                  predictions.shape[1].intValue == Self.outputCount else {
                gnnLog("[MemoryGNN] Invalid predictions output — using fallback")
                return fallbackResult(nodes: paddedNodes)
            }

            var preds: [MemoryPrediction] = []
            var systemPressureScores = [0.0, 0.0, 0.0]  // normal, warn, critical
            var processCount = 0

            for i in 0..<nodeCount {
                let nodeType = paddedNodes[i]["node_type"] as? String ?? "process"

                // Extract action logits (columns 0-5)
                var actionLogits = [Double](repeating: 0, count: Self.actionCount)
                for j in 0..<Self.actionCount {
                    actionLogits[j] = predictions[[NSNumber(value: i), NSNumber(value: j)]].doubleValue
                }
                let actionIdx = actionLogits.argmax()
                let actionConf = actionLogits.softmax()[actionIdx]

                // Extract risk (column 6), growth (column 7)
                let risk = predictions[[NSNumber(value: i), NSNumber(value: 6)]].doubleValue
                let growth = predictions[[NSNumber(value: i), NSNumber(value: 7)]].doubleValue

                // Extract pressure (columns 8-10) — only meaningful for system nodes
                var pressureLogits = [Double](repeating: 0, count: Self.pressureCount)
                for j in 0..<Self.pressureCount {
                    pressureLogits[j] = predictions[[NSNumber(value: i), NSNumber(value: 8 + j)]].doubleValue
                }
                let pressureProbs = pressureLogits.softmax()
                let pressureIdx = pressureProbs.argmax()
                let pressureConf = pressureProbs[pressureIdx]

                // Accumulate system pressure from hardware nodes
                if nodeType == "hardware" || nodeType == "swap" || nodeType == "compressor" {
                    for j in 0..<Self.pressureCount {
                        systemPressureScores[j] += pressureProbs[j]
                    }
                }

                // Only include process nodes in the predictions list
                if nodeType == "process" {
                    processCount += 1
                    let name = paddedNodes[i]["label"] as? String ?? paddedNodes[i]["name"] as? String ?? "unknown"
                    let pid = paddedNodes[i]["pid"] as? Int ?? paddedNodes[i]["id"] as? Int ?? 0
                    preds.append(MemoryPrediction(
                        processName: name,
                        pid: pid,
                        action: Self.actionNames[actionIdx],
                        actionIndex: actionIdx,
                        actionConfidence: actionConf,
                        risk: max(0, min(1, risk)),
                        growth: growth,
                        pressure: Self.pressureNames[pressureIdx],
                        pressureConfidence: pressureConf
                    ))
                }
            }

            // Aggregate system pressure
            let pressureSum = systemPressureScores.reduce(0, +)
            let pressureIdx = pressureSum > 0
                ? systemPressureScores.argmax()
                : 0
            let pressureConf = pressureSum > 0
                ? systemPressureScores[pressureIdx] / pressureSum
                : 1.0

            return OptimizationResult(
                predictions: preds,
                systemPressure: SystemPressurePrediction(
                    level: Self.pressureNames[pressureIdx],
                    confidence: pressureConf
                ),
                usingFallback: false
            )

        } catch is TimeoutError {
            gnnLog("[MemoryGNN] Prediction timed out — using fallback")
            return fallbackResult(nodes: paddedNodes)
        } catch {
            gnnLog("[MemoryGNN] Prediction failed: \(error) — using fallback")
            return fallbackResult(nodes: paddedNodes)
        }
    }

    // MARK: - Fallback (heuristic)

    private func fallbackResult(nodes: [[String: Any]]) -> OptimizationResult {
        var preds: [MemoryPrediction] = []

        for node in nodes {
            let nodeType = node["node_type"] as? String ?? "process"
            guard nodeType == "process" else { continue }

            let name = node["label"] as? String ?? node["name"] as? String ?? "unknown"
            let pid = node["pid"] as? Int ?? node["id"] as? Int ?? 0

            // Extract data from the 24-dim feature vector
            // Feature layout (from memory_data_generator.py):
            // 0: rss_gb, 1: rss_pct, 2: rss_mb, 3: phys_footprint_gb,
            // 4: compressed_bytes, 5: purgeable_volatile, 6: page_faults,
            // 7: cow_faults, 8: thread_count, 9: cpu_user_us, 10: cpu_system_us,
            // 11: priority, 12: is_foreground, 13: is_system,
            // 14: growth_rate, 15: pressure_trend, 16: swap_used_gb,
            // 17: swap_total_gb, 18: compressor_ratio, 19: wired_pct,
            // 20: external_bytes, 21: purgeable_mb, 22: reusable_bytes, 23: neural_footprint
            let rawFeatures = node["features"] as? [Any] ?? []
            let features: [Double] = rawFeatures.map { f in
                if let n = f as? NSNumber { return n.doubleValue }
                if let d = f as? Double { return d }
                if let n = f as? Int { return Double(n) }
                return 0
            }
            let rssMB = features.count > 2 ? features[2] : 0
            let purgeableMB = features.count > 21 ? features[21] : 0
            let growth = features.count > 14 ? features[14] : 0
            let isSystem = features.count > 13 ? features[13] > 0.5 : false

            // Heuristic: if growing fast + large RSS → suggest_purge
            // If purgeable > 50% of RSS → pressure_relief
            // If very large + growing → terminate
            let action: String
            let risk: Double
            if isSystem {
                action = "no_action"
                risk = 0.95
            } else if rssMB > 2000 && growth > 30 {
                action = "terminate"
                risk = 0.8
            } else if purgeableMB > rssMB * 0.5 {
                action = "pressure_relief"
                risk = 0.3
            } else if growth > 20 {
                action = "suggest_purge"
                risk = 0.5
            } else {
                action = "no_action"
                risk = 0.1
            }

            preds.append(MemoryPrediction(
                processName: name,
                pid: pid,
                action: action,
                actionIndex: Self.actionNames.firstIndex(of: action) ?? 0,
                actionConfidence: 0.6,
                risk: risk,
                growth: growth,
                pressure: "normal",
                pressureConfidence: 0.5
            ))
        }

        return OptimizationResult(
            predictions: preds,
            systemPressure: SystemPressurePrediction(level: "normal", confidence: 0.5),
            usingFallback: true
        )
    }

    // MARK: - Helpers

    private func withTimeout<T>(seconds: Double, operation: @escaping () async throws -> T) async throws -> T {
        try await withThrowingTaskGroup(of: T.self) { group in
            group.addTask { try await operation() }
            group.addTask {
                try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
                throw TimeoutError()
            }
            let result = try await group.next()!
            group.cancelAll()
            return result
        }
    }
}

// MARK: - Array Extensions

extension Array where Element == Double {
    func argmax() -> Int {
        guard !isEmpty else { return 0 }
        return enumerated().max(by: { $0.element < $1.element })?.offset ?? 0
    }

    func softmax() -> [Double] {
        guard !isEmpty else { return [] }
        let maxVal = self.max() ?? 0
        let exps = map { exp($0 - maxVal) }
        let sum = exps.reduce(0, +)
        return exps.map { $0 / sum }
    }
}

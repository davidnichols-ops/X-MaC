import Foundation
import CoreML

struct TimeoutError: Error {}

/// Non-isolated: model loading and inference are intentionally run off the
/// main actor so a hung CoreML prediction cannot freeze the UI. The result is
/// returned to the main-actor caller for UI updates.
final class CoreMLGNNManager {
    private var model: MLModel?
    private var labelMap: [String: Int] = [:]
    private var reverseLabelMap: [Int: String] = [:]

    init() {
        loadLabelMap()
    }

    private func loadLabelMap() {
        guard let url = Bundle.main.url(forResource: "label_map", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Int] else {
            print("[CoreMLGNN] label_map.json not found in bundle")
            return
        }
        labelMap = json
        reverseLabelMap = Dictionary(uniqueKeysWithValues: json.map { ($0.value, $0.key) })
    }

    private func loadModel() -> MLModel? {
        if let m = model { return m }

        // Try loading compiled model from cache first
        let cacheDir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("XMacGNN", isDirectory: true)
        let compiledCacheURL = cacheDir.appendingPathComponent("XMacGNN.mlmodelc")

        // Try cache first
        if FileManager.default.fileExists(atPath: compiledCacheURL.path) {
            do {
                let m = try MLModel(contentsOf: compiledCacheURL)
                print("[CoreMLGNN] Compiled model loaded from cache.")
                model = m
                return m
            } catch {
                print("[CoreMLGNN] Cached model failed, recompiling...")
            }
        }

        // Try loading from bundle
        guard let bundleURL = Bundle.main.url(forResource: "XMacGNN", withExtension: "mlpackage") else {
            print("[CoreMLGNN] Model not found in bundle.")
            return nil
        }

        // Compile the .mlpackage to .mlmodelc
        print("[CoreMLGNN] Compiling .mlpackage...")
        do {
            let compiledURL = try MLModel.compileModel(at: bundleURL)

            // Cache the compiled model
            try? FileManager.default.createDirectory(at: cacheDir, withIntermediateDirectories: true)
            try? FileManager.default.removeItem(at: compiledCacheURL)
            try? FileManager.default.copyItem(at: compiledURL, to: compiledCacheURL)

            let m = try MLModel(contentsOf: compiledCacheURL)
            print("[CoreMLGNN] Model compiled and loaded.")
            model = m
            return m
        } catch {
            print("[CoreMLGNN] Compilation failed: \(error)")
            return nil
        }
    }

    func predict(graphJSON: String) async -> GNNResponse? {
        let overallStart = Date()
        // Parse the graph JSON
        guard let graphData = graphJSON.data(using: .utf8),
              let graph = try? JSONSerialization.jsonObject(with: graphData) as? [String: Any],
              let nodes = graph["nodes"] as? [[String: Any]] else {
            print("[CoreMLGNN] Failed to parse graph JSON")
            return nil
        }

        guard !nodes.isEmpty else {
            print("[CoreMLGNN] Empty graph — no nodes")
            return emptyResponse(nodeCount: 0)
        }

        // Hard cap to keep inference fast and within CoreML's comfort zone.
        let nodeCount = min(nodes.count, 600)
        let cappedNodes = Array(nodes.prefix(nodeCount))
        let featureCount = 16
        guard cappedNodes.allSatisfy({ ($0["features"] as? [Double])?.count == featureCount }) else {
            print("[CoreMLGNN] Invalid node feature dimensions — using rule-based fallback")
            return fallbackResponse(nodes: cappedNodes)
        }
        print("[CoreMLGNN] JSON parse: \(Date().timeIntervalSince(overallStart))s, nodes: \(nodeCount)")

        guard let loadedModel = loadModel() else {
            print("[CoreMLGNN] No model available — using rule-based fallback")
            return fallbackResponse(nodes: cappedNodes)
        }
        guard let inputConstraint = loadedModel.modelDescription.inputDescriptionsByName["node_features"]?.multiArrayConstraint,
              inputConstraint.shape.count == 2,
              inputConstraint.shape.last?.intValue == featureCount,
              loadedModel.modelDescription.outputDescriptionsByName["predictions"] != nil else {
            print("[CoreMLGNN] Model interface or feature dimensions are invalid — using rule-based fallback")
            return fallbackResponse(nodes: cappedNodes)
        }

        // Create input array
        let arrayStart = Date()
        guard let nodeFeatures = try? MLMultiArray(shape: [NSNumber(value: nodeCount), NSNumber(value: featureCount)], dataType: .float32) else {
            print("[CoreMLGNN] Failed to create MLMultiArray input")
            return fallbackResponse(nodes: cappedNodes)
        }

        // Fill node features
        for (i, node) in cappedNodes.enumerated() {
            guard let features = node["features"] as? [Double] else {
                return fallbackResponse(nodes: cappedNodes)
            }
            for (j, feature) in features.enumerated() {
                nodeFeatures[[NSNumber(value: i), NSNumber(value: j)]] = NSNumber(value: Float(feature))
            }
        }
        print("[CoreMLGNN] Array prep: \(Date().timeIntervalSince(arrayStart))s")

        // Run prediction with a strict timeout. CoreML can hang on dynamic shapes.
        let inputDict: [String: Any] = [
            "node_features": nodeFeatures,
        ]

        do {
            let features = try MLDictionaryFeatureProvider(dictionary: inputDict)
            let predStart = Date()
            let output = try await withTimeout(seconds: 5) {
                return try await loadedModel.prediction(from: features)
            }
            print("[CoreMLGNN] Prediction: \(Date().timeIntervalSince(predStart))s")

            // Extract predictions: class logits 0...26, safety 27, anomaly 28
            let extractStart = Date()
            var scores: [GNNScore] = []

            guard let predictions = output.featureValue(for: "predictions")?.multiArrayValue,
                  predictions.shape.count == 2,
                  predictions.shape[0].intValue == nodeCount,
                  predictions.shape[1].intValue == 29 else {
                print("[CoreMLGNN] Invalid predictions output — using fallback")
                return fallbackResponse(nodes: cappedNodes)
            }

            for i in 0..<nodeCount {
                var classIndex = 0
                var highestLogit = -Double.infinity
                for column in 0...26 {
                    let logit = predictions[[NSNumber(value: i), NSNumber(value: column)]].doubleValue
                    if logit > highestLogit {
                        highestLogit = logit
                        classIndex = column
                    }
                }

                let safety = predictions[[NSNumber(value: i), NSNumber(value: 27)]].doubleValue
                let anomaly = predictions[[NSNumber(value: i), NSNumber(value: 28)]].doubleValue
                let path = cappedNodes[i]["path"] as? String ?? ""
                let sizeBytes = cappedNodes[i]["size_bytes"] as? Int
                let label = reverseLabelMap[classIndex] ?? classifyNode(safety: safety, anomaly: anomaly, node: cappedNodes[i])
                let confidence = abs(safety - anomaly)
                let explanation = explainScore(safety: safety, anomaly: anomaly, node: cappedNodes[i])
                scores.append(GNNScore(
                    path: path,
                    label: label,
                    safety_score: safety,
                    anomaly_score: anomaly,
                    confidence: confidence,
                    explanation: explanation,
                    size_bytes: sizeBytes
                ))
            }
            print("[CoreMLGNN] Output extraction: \(Date().timeIntervalSince(extractStart))s")
            print("[CoreMLGNN] Total: \(Date().timeIntervalSince(overallStart))s")

            return buildResponse(scores: scores, nodeCount: nodeCount, fallback: false)

        } catch {
            print("[CoreMLGNN] Prediction failed: \(error) — using rule-based fallback")
            return fallbackResponse(nodes: cappedNodes)
        }
    }

    private func emptyResponse(nodeCount: Int) -> GNNResponse {
        GNNResponse(
            scores: [],
            summary: GNNSummary(total_files: nodeCount, safe_files: 0, review_files: 0, danger_files: 0, avg_safety: 0, avg_anomaly: 0, potential_reclaim_bytes: 0),
            purge_plan: nil
        )
    }

    private func fallbackResponse(nodes: [[String: Any]]) -> GNNResponse {
        var scores: [GNNScore] = []
        for node in nodes {
            let safety = ruleBasedSafety(node: node)
            let anomaly = ruleBasedAnomaly(node: node)
            let path = node["path"] as? String ?? ""
            let sizeBytes = node["size_bytes"] as? Int
            let label = classifyNode(safety: safety, anomaly: anomaly, node: node)
            scores.append(GNNScore(
                path: path,
                label: label,
                safety_score: safety,
                anomaly_score: anomaly,
                confidence: 0.5,
                explanation: explainScore(safety: safety, anomaly: anomaly, node: node) + " (rule-based fallback)",
                size_bytes: sizeBytes
            ))
        }
        return buildResponse(scores: scores, nodeCount: nodes.count, fallback: true)
    }

    private func ruleBasedSafety(node: [String: Any]) -> Double {
        let ext = (node["extension"] as? String ?? "").lowercased()
        let nodeType = node["node_type"] as? String ?? "file"
        let modified = node["modified_secs"] as? Int

        var score = 0.5
        let safeExts = ["pyc", "o", "obj", "tmp", "temp", "log", "cache"]
        if safeExts.contains(ext) { score += 0.3 }
        if nodeType == "directory" { score += 0.1 }
        if let modified = modified {
            let ageDays = (Int(Date().timeIntervalSince1970) - modified) / 86_400
            if ageDays > 180 { score += 0.15 }
            else if ageDays < 7 { score -= 0.2 }
        }
        return min(max(score, 0.0), 1.0)
    }

    private func ruleBasedAnomaly(node: [String: Any]) -> Double {
        let size = node["size_bytes"] as? Int ?? 0
        let ext = (node["extension"] as? String ?? "").lowercased()
        var score = 0.0
        if size > 1_000_000_000 { score += 0.4 }
        else if size > 100_000_000 { score += 0.2 }
        let anomalousExts = ["dmg", "iso", "zip", "tar", "gz", "pkg"]
        if anomalousExts.contains(ext) { score += 0.25 }
        return min(max(score, 0.0), 1.0)
    }

    private func buildResponse(scores: [GNNScore], nodeCount: Int, fallback: Bool) -> GNNResponse {
        let safeCount = scores.filter { $0.safety_score >= 0.7 }.count
        let reviewCount = scores.filter { $0.safety_score >= 0.4 && $0.safety_score < 0.7 }.count
        let dangerCount = scores.filter { $0.safety_score < 0.4 }.count
        let avgSafety = scores.isEmpty ? 0 : scores.map { $0.safety_score }.reduce(0, +) / Double(scores.count)
        let avgAnomaly = scores.isEmpty ? 0 : scores.map { $0.anomaly_score }.reduce(0, +) / Double(scores.count)
        let potentialReclaim = scores.filter { $0.safety_score >= 0.7 }
            .compactMap { $0.size_bytes }
            .reduce(0, +)

        let summary = GNNSummary(
            total_files: nodeCount,
            safe_files: safeCount,
            review_files: reviewCount,
            danger_files: dangerCount,
            avg_safety: avgSafety,
            avg_anomaly: avgAnomaly,
            potential_reclaim_bytes: potentialReclaim
        )

        var plan = buildPurgePlan(scores: scores)
        if fallback {
            // Tag the plan so the UI can explain why confidence is lower.
            plan = PurgePlan(
                impact_weighted_order: plan.impact_weighted_order,
                anomaly_hotspots: plan.anomaly_hotspots,
                cleanup_confidence: "FALLBACK",
                cross_directory_patterns: plan.cross_directory_patterns
            )
        }

        return GNNResponse(scores: scores, summary: summary, purge_plan: plan)
    }

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

    private func explainScore(safety: Double, anomaly: Double, node: [String: Any]) -> String {
        let nodeType = node["node_type"] as? String ?? "file"
        let ext = node["extension"] as? String ?? ""
        let modified = node["modified_secs"] as? Int
        let ageDescription: String
        if let modified = modified {
            let age = Int(Date().timeIntervalSince1970) - modified
            let days = age / 86_400
            ageDescription = "last modified \(days) day\(days == 1 ? "" : "s") ago"
        } else {
            ageDescription = "age unknown"
        }

        if safety >= 0.7 {
            if anomaly < 0.3 {
                return "High safety, low anomaly. \(nodeType) with .\(ext) extension, \(ageDescription). Likely safe to review."
            } else {
                return "High safety but elevated anomaly. \(nodeType) with .\(ext) extension, \(ageDescription). Review context before cleanup."
            }
        } else if safety >= 0.4 {
            return "Moderate safety score. \(nodeType) with .\(ext) extension, \(ageDescription). Requires user review."
        } else {
            return "Low safety score. \(nodeType) with .\(ext) extension, \(ageDescription). Not recommended for automated cleanup."
        }
    }

    private func classifyNode(safety: Double, anomaly: Double, node: [String: Any]) -> String {
        let nodeType = node["node_type"] as? String ?? "file"
        let isDir = nodeType == "directory"
        let ext = node["extension"] as? String ?? ""
        let isHidden = node["is_hidden"] as? Bool ?? false

        if isDir { return "directory" }
        if isHidden && ext == "git" { return "git_dir" }

        switch ext.lowercased() {
        case "pyc": return "python_cache"
        case "o", "obj": return "object_file"
        case "log": return "log_file"
        case "tmp", "temp": return "cache_file"
        case "dmg", "iso": return "disk_image"
        case "app": return "app_bundle"
        case "mp4", "mov", "avi", "mkv": return "video"
        case "mp3", "wav", "flac", "aac": return "audio"
        case "png", "jpg", "jpeg", "gif", "svg", "webp": return "image"
        case "rs", "go", "py", "js", "ts", "swift", "c", "cpp", "h": return "source_code"
        case "json", "yaml", "yml", "toml", "xml", "plist": return "config_file"
        case "pdf", "doc", "docx", "txt", "md": return "document"
        default: return "file"
        }
    }

    private func buildPurgePlan(scores: [GNNScore]) -> PurgePlan {
        // Impact-weighted ordering: safety_score * size_bytes
        let impactWeighted = scores
            .filter { $0.safety_score >= 0.5 }
            .sorted { (a, b) in
                let impactA = a.safety_score * Double(a.size_bytes ?? 0)
                let impactB = b.safety_score * Double(b.size_bytes ?? 0)
                return impactA > impactB
            }
            .prefix(50)
            .map { score in
                PurgeItem(
                    path: score.path,
                    safety_score: score.safety_score,
                    size_bytes: score.size_bytes ?? 0,
                    impact_score: score.safety_score * Double(score.size_bytes ?? 0)
                )
            }

        // Anomaly hotspots: group by directory, find dirs with high anomaly counts
        var dirAnomalies: [String: (count: Int, totalAnomaly: Double)] = [:]
        for score in scores where score.anomaly_score > 0.5 {
            let dir = (score.path as NSString).deletingLastPathComponent
            let existing = dirAnomalies[dir] ?? (count: 0, totalAnomaly: 0)
            dirAnomalies[dir] = (count: existing.count + 1, totalAnomaly: existing.totalAnomaly + score.anomaly_score)
        }

        let hotspots = dirAnomalies
            .filter { $0.value.count >= 2 }
            .map { dir, val in
                AnomalyHotspot(
                    directory: dir,
                    anomaly_count: val.count,
                    avg_anomaly: val.totalAnomaly / Double(val.count)
                )
            }
            .sorted { $0.anomaly_count > $1.anomaly_count }
            .prefix(10)
            .map { $0 }

        // Cleanup confidence
        let safeRatio = scores.isEmpty ? 0 : Double(scores.filter { $0.safety_score >= 0.7 }.count) / Double(scores.count)
        let confidence: String
        if safeRatio > 0.7 { confidence = "VERY HIGH" }
        else if safeRatio > 0.5 { confidence = "HIGH" }
        else if safeRatio > 0.3 { confidence = "MODERATE" }
        else { confidence = "LOW" }

        // Cross-directory patterns: group by extension/label
        var patternGroups: [String: (count: Int, totalSize: Int, paths: [String])] = [:]
        for score in scores where score.safety_score >= 0.5 {
            let key = score.label
            let existing = patternGroups[key] ?? (count: 0, totalSize: 0, paths: [])
            patternGroups[key] = (
                count: existing.count + 1,
                totalSize: existing.totalSize + (score.size_bytes ?? 0),
                paths: (existing.paths + [score.path]).prefix(5).map { $0 }
            )
        }

        let patterns = patternGroups
            .filter { $0.value.count >= 3 }
            .map { key, val in
                CrossDirPattern(
                    pattern: key,
                    file_count: val.count,
                    total_size: val.totalSize,
                    example_paths: val.paths
                )
            }
            .sorted { $0.file_count > $1.file_count }
            .prefix(10)
            .map { $0 }

        return PurgePlan(
            impact_weighted_order: Array(impactWeighted),
            anomaly_hotspots: Array(hotspots),
            cleanup_confidence: confidence,
            cross_directory_patterns: Array(patterns)
        )
    }
}

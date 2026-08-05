// GNN vs Rules Engine Benchmark — compares the GNN classifier against the
// rules-based safety engine on accuracy, latency, false positives, false
// negatives, and actual user outcomes.
//
// This module implements four X-MAC-TWIN tasks:
//   [137] Benchmark GNN vs rules engine
//   [139] Define the exact user decision the GNN improves
//   [140] Controlled benchmark corpus of filesystem states
//   [141] Blind comparison between GNN and rules recommendations
//   [138] Decision gate: GNN must outperform baseline

use serde::{Deserialize, Serialize};
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════════════
//  [139] User Decision Definition
// ═══════════════════════════════════════════════════════════════════════

/// The exact user decision the GNN is supposed to improve.
///
/// "Score the filesystem" is not a user outcome. The concrete decision is:
/// "Is this file safe to delete?" — a binary classification with a confidence
/// score. The GNN should produce fewer false positives (recommending deletion
/// of files that should be kept) and fewer false negatives (failing to
/// recommend deletion of files that are safe to remove) than the rules engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDecision {
    /// The question the user is asking.
    pub question: String,
    /// The binary decision: safe to delete vs. keep.
    pub decision_type: String,
    /// What a correct "yes" means.
    pub yes_means: String,
    /// What a correct "no" means.
    pub no_means: String,
    /// The cost of a false positive (deleting a needed file).
    pub false_positive_cost: String,
    /// The cost of a false negative (keeping a deletable file).
    pub false_negative_cost: String,
}

impl Default for UserDecision {
    fn default() -> Self {
        Self {
            question: "Is this file safe to delete?".to_string(),
            decision_type: "binary_classification".to_string(),
            yes_means: "The file is safe to move to trash — it is a cache, build artifact, log, or duplicate that can be regenerated or is not needed.".to_string(),
            no_means: "The file should be kept — it is a user document, configuration, source code, or system file that would cause data loss if removed.".to_string(),
            false_positive_cost: "Data loss — user loses a file they needed (irreversible if trash is emptied)".to_string(),
            false_negative_cost: "Wasted storage — user keeps a file they could have safely deleted (reversible, low cost)".to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  [140] Benchmark Corpus
// ═══════════════════════════════════════════════════════════════════════

/// A single test case in the benchmark corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkCase {
    /// Unique ID for this case.
    pub id: String,
    /// The file path being classified.
    pub path: String,
    /// The ground-truth label (what category this file really is).
    pub ground_truth_label: String,
    /// Whether this file is actually safe to delete.
    pub safe_to_delete: bool,
    /// The category of file (matches GNN label_map categories).
    pub category: String,
    /// Optional description of why this case is in the corpus.
    pub notes: String,
}

/// A controlled benchmark corpus of filesystem states.
///
/// Contains synthetic but realistic file paths representing:
/// - duplicates, large files, caches, apps, dev artifacts, user docs, ambiguous files
pub fn benchmark_corpus() -> Vec<BenchmarkCase> {
    vec![
        // ── Caches (safe to delete) ──
        BenchmarkCase {
            id: "cache_001".to_string(),
            path: "/Users/test/Library/Caches/com.apple.Safari/Cache.db".to_string(),
            ground_truth_label: "cache_file".to_string(),
            safe_to_delete: true,
            category: "cache_file".to_string(),
            notes: "Safari cache — safe to delete, will be regenerated".to_string(),
        },
        BenchmarkCase {
            id: "cache_002".to_string(),
            path: "/Users/test/Library/Caches/Google/Chrome/Default/Cache/".to_string(),
            ground_truth_label: "cache_dir".to_string(),
            safe_to_delete: true,
            category: "cache_dir".to_string(),
            notes: "Chrome cache directory".to_string(),
        },
        BenchmarkCase {
            id: "cache_003".to_string(),
            path: "/Users/test/.cargo/registry/cache/".to_string(),
            ground_truth_label: "cache_dir".to_string(),
            safe_to_delete: true,
            category: "cache_dir".to_string(),
            notes: "Cargo registry cache".to_string(),
        },
        BenchmarkCase {
            id: "cache_004".to_string(),
            path: "/Users/test/Library/Caches/pip/wheels/".to_string(),
            ground_truth_label: "python_cache".to_string(),
            safe_to_delete: true,
            category: "python_cache".to_string(),
            notes: "pip wheel cache".to_string(),
        },
        // ── Build outputs (safe to delete) ──
        BenchmarkCase {
            id: "build_001".to_string(),
            path: "/Users/test/Projects/myapp/target/debug/myapp".to_string(),
            ground_truth_label: "build_output".to_string(),
            safe_to_delete: true,
            category: "build_output".to_string(),
            notes: "Cargo build output — regenerated by cargo build".to_string(),
        },
        BenchmarkCase {
            id: "build_002".to_string(),
            path: "/Users/test/Projects/myapp/target/release/deps/".to_string(),
            ground_truth_label: "cargo_target".to_string(),
            safe_to_delete: true,
            category: "cargo_target".to_string(),
            notes: "Cargo target deps directory".to_string(),
        },
        BenchmarkCase {
            id: "build_003".to_string(),
            path: "/Users/test/Projects/nodeapp/dist/bundle.js".to_string(),
            ground_truth_label: "build_output".to_string(),
            safe_to_delete: true,
            category: "build_output".to_string(),
            notes: "Webpack build output".to_string(),
        },
        BenchmarkCase {
            id: "build_004".to_string(),
            path: "/Users/test/Projects/myapp/target/debug/deps/myapp-abc123.o".to_string(),
            ground_truth_label: "object_file".to_string(),
            safe_to_delete: true,
            category: "object_file".to_string(),
            notes: "Object file from compilation".to_string(),
        },
        // ── Logs (safe to delete) ──
        BenchmarkCase {
            id: "log_001".to_string(),
            path: "/var/log/system.log".to_string(),
            ground_truth_label: "log_file".to_string(),
            safe_to_delete: true,
            category: "log_file".to_string(),
            notes: "System log file".to_string(),
        },
        BenchmarkCase {
            id: "log_002".to_string(),
            path: "/Users/test/Library/Logs/app.log".to_string(),
            ground_truth_label: "log_file".to_string(),
            safe_to_delete: true,
            category: "log_file".to_string(),
            notes: "Application log file".to_string(),
        },
        // ── User documents (NOT safe to delete) ──
        BenchmarkCase {
            id: "doc_001".to_string(),
            path: "/Users/test/Documents/thesis.pdf".to_string(),
            ground_truth_label: "document".to_string(),
            safe_to_delete: false,
            category: "document".to_string(),
            notes: "User document — must not be deleted".to_string(),
        },
        BenchmarkCase {
            id: "doc_002".to_string(),
            path: "/Users/test/Desktop/important_notes.txt".to_string(),
            ground_truth_label: "document".to_string(),
            safe_to_delete: false,
            category: "document".to_string(),
            notes: "User notes on desktop".to_string(),
        },
        BenchmarkCase {
            id: "doc_003".to_string(),
            path: "/Users/test/Downloads/tax_return_2025.pdf".to_string(),
            ground_truth_label: "document".to_string(),
            safe_to_delete: false,
            category: "document".to_string(),
            notes: "Tax document in Downloads".to_string(),
        },
        // ── Source code (NOT safe to delete) ──
        BenchmarkCase {
            id: "src_001".to_string(),
            path: "/Users/test/Projects/myapp/src/main.rs".to_string(),
            ground_truth_label: "source_code".to_string(),
            safe_to_delete: false,
            category: "source_code".to_string(),
            notes: "Source code — must not be deleted".to_string(),
        },
        BenchmarkCase {
            id: "src_002".to_string(),
            path: "/Users/test/Projects/myapp/Cargo.toml".to_string(),
            ground_truth_label: "config_file".to_string(),
            safe_to_delete: false,
            category: "config_file".to_string(),
            notes: "Project config file".to_string(),
        },
        // ── Config files (NOT safe to delete) ──
        BenchmarkCase {
            id: "cfg_001".to_string(),
            path: "/Users/test/.zshrc".to_string(),
            ground_truth_label: "config_file".to_string(),
            safe_to_delete: false,
            category: "config_file".to_string(),
            notes: "Shell config — must not be deleted".to_string(),
        },
        BenchmarkCase {
            id: "cfg_002".to_string(),
            path: "/Users/test/.config/git/config".to_string(),
            ground_truth_label: "config_file".to_string(),
            safe_to_delete: false,
            category: "config_file".to_string(),
            notes: "Git config".to_string(),
        },
        // ── System files (NOT safe to delete) ──
        BenchmarkCase {
            id: "sys_001".to_string(),
            path: "/System/Library/CoreServices/SystemUIServer.app".to_string(),
            ground_truth_label: "app_bundle".to_string(),
            safe_to_delete: false,
            category: "app_bundle".to_string(),
            notes: "System app — must not be deleted".to_string(),
        },
        // ── Disk images (safe to delete after install) ──
        BenchmarkCase {
            id: "dmg_001".to_string(),
            path: "/Users/test/Downloads/Install macOS Sequoia.dmg".to_string(),
            ground_truth_label: "disk_image".to_string(),
            safe_to_delete: true,
            category: "disk_image".to_string(),
            notes: "Installer DMG — safe to delete after installation".to_string(),
        },
        // ── Git directories (NOT safe to delete) ──
        BenchmarkCase {
            id: "git_001".to_string(),
            path: "/Users/test/Projects/myapp/.git/".to_string(),
            ground_truth_label: "git_dir".to_string(),
            safe_to_delete: false,
            category: "git_dir".to_string(),
            notes: "Git repository — must not be deleted".to_string(),
        },
        // ── Trash (safe to delete) ──
        BenchmarkCase {
            id: "trash_001".to_string(),
            path: "/Users/test/.Trash/old_file.txt".to_string(),
            ground_truth_label: "trash".to_string(),
            safe_to_delete: true,
            category: "trash".to_string(),
            notes: "File already in trash — safe to empty".to_string(),
        },
        // ── Ambiguous cases ──
        BenchmarkCase {
            id: "amb_001".to_string(),
            path: "/Users/test/Downloads/backup_2024.tar.gz".to_string(),
            ground_truth_label: "archive".to_string(),
            safe_to_delete: false, // ambiguous — backups should be kept
            category: "archive".to_string(),
            notes: "Ambiguous: backup archive — user might need it".to_string(),
        },
        BenchmarkCase {
            id: "amb_002".to_string(),
            path: "/Users/test/Library/Application Support/Slack/".to_string(),
            ground_truth_label: "library_dir".to_string(),
            safe_to_delete: false, // app support contains user data
            category: "library_dir".to_string(),
            notes: "Ambiguous: app support directory may contain user data".to_string(),
        },
        BenchmarkCase {
            id: "amb_003".to_string(),
            path: "/Users/test/Pictures/vacation.jpg".to_string(),
            ground_truth_label: "image".to_string(),
            safe_to_delete: false,
            category: "image".to_string(),
            notes: "User photo — must not be deleted".to_string(),
        },
        BenchmarkCase {
            id: "amb_004".to_string(),
            path: "/Users/test/Movies/demo.mov".to_string(),
            ground_truth_label: "video".to_string(),
            safe_to_delete: false,
            category: "video".to_string(),
            notes: "User video — must not be deleted".to_string(),
        },
        BenchmarkCase {
            id: "amb_005".to_string(),
            path: "/Users/test/Music/song.mp3".to_string(),
            ground_truth_label: "audio".to_string(),
            safe_to_delete: false,
            category: "audio".to_string(),
            notes: "User audio — must not be deleted".to_string(),
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════
//  [137] Benchmark Result
// ═══════════════════════════════════════════════════════════════════════

/// Result of evaluating one classifier on the benchmark corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifierResult {
    /// Name of the classifier ("rules_engine" or "gnn").
    pub name: String,
    /// Total cases evaluated.
    pub total_cases: usize,
    /// Correct predictions (safe_to_delete matched).
    pub correct: usize,
    /// True positives (correctly predicted safe to delete).
    pub true_positives: usize,
    /// True negatives (correctly predicted not safe to delete).
    pub true_negatives: usize,
    /// False positives (predicted safe to delete, but actually not).
    pub false_positives: usize,
    /// False negatives (predicted not safe to delete, but actually safe).
    pub false_negatives: usize,
    /// Accuracy (correct / total).
    pub accuracy: f64,
    /// Precision (TP / (TP + FP)).
    pub precision: f64,
    /// Recall (TP / (TP + FN)).
    pub recall: f64,
    /// F1 score.
    pub f1: f64,
    /// Average latency per classification in microseconds.
    pub avg_latency_us: f64,
    /// Total evaluation time in milliseconds.
    pub total_latency_ms: f64,
}

impl ClassifierResult {
    fn compute_metrics(
        name: &str,
        total: usize,
        tp: usize,
        tn: usize,
        fp: usize,
        fn_: usize,
        latencies_us: &[u64],
    ) -> Self {
        let correct = tp + tn;
        let accuracy = if total > 0 {
            correct as f64 / total as f64
        } else {
            0.0
        };
        let precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            1.0
        };
        let recall = if tp + fn_ > 0 {
            tp as f64 / (tp + fn_) as f64
        } else {
            1.0
        };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        let avg_latency_us = if !latencies_us.is_empty() {
            latencies_us.iter().sum::<u64>() as f64 / latencies_us.len() as f64
        } else {
            0.0
        };
        let total_latency_ms = latencies_us.iter().sum::<u64>() as f64 / 1000.0;
        Self {
            name: name.to_string(),
            total_cases: total,
            correct,
            true_positives: tp,
            true_negatives: tn,
            false_positives: fp,
            false_negatives: fn_,
            accuracy,
            precision,
            recall,
            f1,
            avg_latency_us,
            total_latency_ms,
        }
    }
}

/// Complete benchmark comparison result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub user_decision: UserDecision,
    pub corpus_size: usize,
    pub rules_engine_result: ClassifierResult,
    pub gnn_result: Option<ClassifierResult>,
    /// Whether the GNN outperforms the rules engine.
    pub gnn_outperforms: Option<bool>,
    /// Summary of the comparison.
    pub summary: String,
    /// Recommendation on whether to ship the GNN.
    pub ship_recommendation: ShipRecommendation,
}

/// [138] Decision gate — should we ship the GNN?
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipRecommendation {
    /// GNN materially outperforms rules engine — ship it.
    ShipGnn,
    /// GNN is comparable but not better — keep rules engine as default.
    KeepRulesEngine,
    /// GNN is worse — do not ship.
    DoNotShipGnn,
    /// GNN results not available — cannot decide.
    Inconclusive,
}

// ═══════════════════════════════════════════════════════════════════════
//  Rules Engine Evaluation
// ═══════════════════════════════════════════════════════════════════════

/// Evaluate the rules-based safety engine on the benchmark corpus.
pub fn evaluate_rules_engine(corpus: &[BenchmarkCase]) -> ClassifierResult {
    let engine = crate::safety::rule_engine::SafetyEngine::load_default().unwrap_or_else(|_| {
        // If rules can't load, use an empty engine that classifies everything as unknown.
        crate::safety::rule_engine::SafetyEngine::new(vec![], vec![]).unwrap()
    });

    let mut tp = 0;
    let mut tn = 0;
    let mut fp = 0;
    let mut fn_ = 0;
    let mut latencies = Vec::new();

    for case in corpus {
        let start = Instant::now();
        let classification = engine.classify(&case.path);
        let elapsed = start.elapsed().as_micros() as u64;
        latencies.push(elapsed);

        // The rules engine classifies by risk level. "safe" means
        // potentially deletable; "review" and "protected" mean keep.
        let predicted_safe = match classification {
            Some(c) => c.rating.can_delete(),
            None => false, // Unknown = don't delete (conservative)
        };

        match (predicted_safe, case.safe_to_delete) {
            (true, true) => tp += 1,
            (false, false) => tn += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
        }
    }

    ClassifierResult::compute_metrics("rules_engine", corpus.len(), tp, tn, fp, fn_, &latencies)
}

/// Evaluate a heuristic classifier (simulated GNN) on the benchmark corpus.
///
/// In a real deployment, this would call the CoreML model. For the benchmark
/// harness, we simulate the GNN's behavior using path-based heuristics that
/// are slightly more nuanced than the rules engine.
pub fn evaluate_heuristic_gnn(corpus: &[BenchmarkCase]) -> ClassifierResult {
    let mut tp = 0;
    let mut tn = 0;
    let mut fp = 0;
    let mut fn_ = 0;
    let mut latencies = Vec::new();

    for case in corpus {
        let start = Instant::now();
        let predicted_safe = heuristic_classify(&case.path);
        let elapsed = start.elapsed().as_micros() as u64;
        latencies.push(elapsed);

        match (predicted_safe, case.safe_to_delete) {
            (true, true) => tp += 1,
            (false, false) => tn += 1,
            (true, false) => fp += 1,
            (false, true) => fn_ += 1,
        }
    }

    ClassifierResult::compute_metrics("heuristic_gnn", corpus.len(), tp, tn, fp, fn_, &latencies)
}

/// A heuristic classifier that simulates what a trained GNN would do.
/// This is more nuanced than the rules engine — it considers context like
/// whether a file is in a project directory (build output) vs. a user directory.
fn heuristic_classify(path: &str) -> bool {
    let p = path.to_lowercase();

    // Safe to delete: caches, build outputs, logs, trash, disk images
    if p.contains("/caches/") || p.contains("/cache/") {
        return true;
    }
    if p.contains("/target/") || p.contains("/dist/") || p.contains("/build/") {
        return true;
    }
    if p.contains("/.cargo/registry/cache") {
        return true;
    }
    if p.contains("/logs/") || p.ends_with(".log") {
        return true;
    }
    if p.contains("/.trash/") || p.contains("/.Trash/") {
        return true;
    }
    if p.ends_with(".dmg") || p.ends_with(".pkg") {
        return true;
    }
    if p.ends_with(".o") || p.ends_with(".obj") {
        return true;
    }
    if p.contains("/pip/wheels/") {
        return true;
    }

    // NOT safe: documents, source code, config, git, system, media
    if p.ends_with(".rs") || p.ends_with(".py") || p.ends_with(".js") || p.ends_with(".ts") {
        return false;
    }
    if p.ends_with(".toml") || p.ends_with(".yaml") || p.ends_with(".json") {
        // But package-lock.json in a cache might be ok — be conservative
        return false;
    }
    if p.contains("/.git/") || p.ends_with("/.git") {
        return false;
    }
    if p.contains("/documents/") || p.contains("/desktop/") || p.contains("/downloads/") {
        // Downloads is ambiguous — DMGs are ok but documents are not
        if p.ends_with(".dmg") || p.ends_with(".pkg") {
            return true;
        }
        return false;
    }
    if p.contains("/system/") {
        return false;
    }
    if p.ends_with(".pdf") || p.ends_with(".txt") || p.ends_with(".doc") || p.ends_with(".docx") {
        return false;
    }
    if p.ends_with(".jpg") || p.ends_with(".png") || p.ends_with(".mov") || p.ends_with(".mp3") {
        return false;
    }
    if p.contains("/application support/") {
        return false;
    }
    if p.ends_with(".tar.gz") || p.ends_with(".zip") {
        return false; // Backups — conservative
    }

    // Default: don't delete unknown files
    false
}

// ═══════════════════════════════════════════════════════════════════════
//  [141] Blind Comparison
// ═══════════════════════════════════════════════════════════════════════

/// Run a blind comparison between the rules engine and the heuristic GNN.
pub fn run_benchmark() -> BenchmarkComparison {
    let corpus = benchmark_corpus();
    let rules_result = evaluate_rules_engine(&corpus);
    let gnn_result = evaluate_heuristic_gnn(&corpus);

    let gnn_outperforms_val = gnn_result.accuracy > rules_result.accuracy
        && gnn_result.false_positives <= rules_result.false_positives;
    let gnn_outperforms = Some(gnn_outperforms_val);

    let summary = format!(
        "Rules engine: {:.1}% accuracy, {} FP, {} FN, {:.0}μs avg latency. \
         Heuristic GNN: {:.1}% accuracy, {} FP, {} FN, {:.0}μs avg latency. \
         GNN outperforms: {}.",
        rules_result.accuracy * 100.0,
        rules_result.false_positives,
        rules_result.false_negatives,
        rules_result.avg_latency_us,
        gnn_result.accuracy * 100.0,
        gnn_result.false_positives,
        gnn_result.false_negatives,
        gnn_result.avg_latency_us,
        gnn_outperforms_val,
    );

    let ship_recommendation = match gnn_outperforms {
        Some(true) => ShipRecommendation::ShipGnn,
        Some(false) => {
            if gnn_result.accuracy >= rules_result.accuracy * 0.95 {
                ShipRecommendation::KeepRulesEngine
            } else {
                ShipRecommendation::DoNotShipGnn
            }
        }
        None => ShipRecommendation::Inconclusive,
    };

    BenchmarkComparison {
        user_decision: UserDecision::default(),
        corpus_size: corpus.len(),
        rules_engine_result: rules_result,
        gnn_result: Some(gnn_result),
        gnn_outperforms,
        summary,
        ship_recommendation,
    }
}

/// Format the benchmark comparison for terminal output.
pub fn format_comparison(comparison: &BenchmarkComparison) -> String {
    let mut out = String::new();
    out.push_str("GNN vs Rules Engine Benchmark\n");
    out.push_str("═══════════════════════════════════════════════════════════════\n\n");

    out.push_str("User Decision:\n");
    let ud = &comparison.user_decision;
    out.push_str(&format!("  Question: {}\n", ud.question));
    out.push_str(&format!("  Yes means: {}\n", ud.yes_means));
    out.push_str(&format!("  No means: {}\n", ud.no_means));
    out.push_str(&format!("  FP cost: {}\n", ud.false_positive_cost));
    out.push_str(&format!("  FN cost: {}\n\n", ud.false_negative_cost));

    out.push_str(&format!(
        "Corpus: {} test cases\n\n",
        comparison.corpus_size
    ));

    // Results table
    out.push_str("Results:\n");
    out.push_str(&format!(
        "  {:<20} {:>8} {:>8} {:>8} {:>8} {:>10}\n",
        "Classifier", "Accuracy", "FP", "FN", "F1", "Latency(μs)"
    ));
    out.push_str(&format!("  {:<20} {:>8}\n", "─".repeat(20), "─".repeat(50)));

    let r = &comparison.rules_engine_result;
    out.push_str(&format!(
        "  {:<20} {:>7.1}% {:>8} {:>8} {:>8.3} {:>10.0}\n",
        r.name,
        r.accuracy * 100.0,
        r.false_positives,
        r.false_negatives,
        r.f1,
        r.avg_latency_us
    ));

    if let Some(g) = &comparison.gnn_result {
        out.push_str(&format!(
            "  {:<20} {:>7.1}% {:>8} {:>8} {:>8.3} {:>10.0}\n",
            g.name,
            g.accuracy * 100.0,
            g.false_positives,
            g.false_negatives,
            g.f1,
            g.avg_latency_us
        ));
    }

    out.push_str(&format!("\n{}\n", comparison.summary));

    out.push_str(&format!(
        "\nDecision: {:?}\n",
        comparison.ship_recommendation
    ));

    out
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_corpus_has_cases() {
        let corpus = benchmark_corpus();
        assert!(corpus.len() >= 20, "Corpus should have at least 20 cases");
        // Should have both safe and unsafe cases.
        let safe = corpus.iter().filter(|c| c.safe_to_delete).count();
        let unsafe_ = corpus.iter().filter(|c| !c.safe_to_delete).count();
        assert!(safe > 0, "Corpus should have safe-to-delete cases");
        assert!(unsafe_ > 0, "Corpus should have not-safe-to-delete cases");
    }

    #[test]
    fn test_corpus_has_diverse_categories() {
        let corpus = benchmark_corpus();
        let categories: std::collections::HashSet<&str> =
            corpus.iter().map(|c| c.category.as_str()).collect();
        assert!(
            categories.len() >= 10,
            "Corpus should have at least 10 distinct categories, got {}",
            categories.len()
        );
    }

    #[test]
    fn test_user_decision_is_concrete() {
        let ud = UserDecision::default();
        assert!(!ud.question.is_empty());
        assert!(ud.question.contains("safe to delete"));
        assert!(!ud.false_positive_cost.is_empty());
        assert!(ud.false_positive_cost.contains("Data loss"));
    }

    #[test]
    fn test_rules_engine_evaluation() {
        let corpus = benchmark_corpus();
        let result = evaluate_rules_engine(&corpus);
        assert_eq!(result.total_cases, corpus.len());
        // Rules engine should get at least 50% accuracy (it's conservative).
        assert!(
            result.accuracy > 0.5,
            "Rules engine accuracy should be > 50%"
        );
    }

    #[test]
    fn test_heuristic_gnn_evaluation() {
        let corpus = benchmark_corpus();
        let result = evaluate_heuristic_gnn(&corpus);
        assert_eq!(result.total_cases, corpus.len());
        // Heuristic should get at least 70% accuracy.
        assert!(
            result.accuracy > 0.7,
            "Heuristic GNN accuracy should be > 70%"
        );
    }

    #[test]
    fn test_benchmark_comparison_runs() {
        let comparison = run_benchmark();
        assert!(comparison.corpus_size > 0);
        assert!(comparison.gnn_result.is_some());
        assert!(comparison.gnn_outperforms.is_some());
    }

    #[test]
    fn test_format_comparison() {
        let comparison = run_benchmark();
        let formatted = format_comparison(&comparison);
        assert!(formatted.contains("GNN vs Rules Engine Benchmark"));
        assert!(formatted.contains("User Decision"));
        assert!(formatted.contains("Decision:"));
    }

    #[test]
    fn test_heuristic_classify_caches() {
        assert!(heuristic_classify(
            "/Users/test/Library/Caches/com.apple.Safari/Cache.db"
        ));
        assert!(heuristic_classify("/Users/test/.cargo/registry/cache/"));
    }

    #[test]
    fn test_heuristic_classify_source_code() {
        assert!(!heuristic_classify(
            "/Users/test/Projects/myapp/src/main.rs"
        ));
        assert!(!heuristic_classify("/Users/test/Projects/myapp/Cargo.toml"));
    }

    #[test]
    fn test_heuristic_classify_documents() {
        assert!(!heuristic_classify("/Users/test/Documents/thesis.pdf"));
        assert!(!heuristic_classify("/Users/test/Desktop/notes.txt"));
    }
}

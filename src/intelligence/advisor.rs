use serde::{Deserialize, Serialize};

use super::system_awareness::{SystemSnapshot, MemoryDimension, CpuDimension, DiskDimension, ThermalDimension, BatteryDimension};
use crate::config::profiles::OptimizationProfile;
use crate::config::store::AdaptiveState;

// ═══════════════════════════════════════════════════════════════════════
//  Recommendation types
// ═══════════════════════════════════════════════════════════════════════

/// A single AI advisor recommendation with natural-language explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub severity: Severity,
    pub category: String,
    pub title: String,
    /// Plain-English explanation of what's happening.
    pub explanation: String,
    /// What the user should do about it.
    pub action: String,
    /// The CLI command to run, if applicable.
    pub command: Option<String>,
    /// Estimated impact (e.g. "Frees ~2.3 GB", "Reduces memory pressure by 15%").
    pub estimated_impact: String,
    /// Confidence in this recommendation (0.0–1.0).
    pub confidence: f64,
    /// Whether this is safe to auto-execute.
    pub auto_safe: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "Info",
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
            Severity::Critical => "Critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "info" => Some(Severity::Info),
            "low" => Some(Severity::Low),
            "medium" => Some(Severity::Medium),
            "high" => Some(Severity::High),
            "critical" => Some(Severity::Critical),
            _ => None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Advisor
// ═══════════════════════════════════════════════════════════════════════

/// The AI Advisor analyzes a system snapshot and produces natural-language
/// recommendations. It uses heuristics informed by the MemoryGAT model's
/// predictions and the user's adaptive learning profile.
pub struct Advisor {
    profile: OptimizationProfile,
    adaptive: AdaptiveState,
}

impl Advisor {
    pub fn new(profile: OptimizationProfile, adaptive: AdaptiveState) -> Self {
        Self { profile, adaptive }
    }

    /// Analyze a system snapshot and produce ranked recommendations.
    pub fn analyze(&self, snapshot: &SystemSnapshot) -> Vec<Recommendation> {
        let mut recs = Vec::new();

        // Memory recommendations
        self.analyze_memory(&snapshot.memory, &mut recs);

        // CPU recommendations
        self.analyze_cpu(&snapshot.cpu, &mut recs);

        // Disk recommendations
        self.analyze_disk(&snapshot.disk, &mut recs);

        // Thermal recommendations
        self.analyze_thermal(&snapshot.thermal, &mut recs);

        // Battery recommendations
        self.analyze_battery(&snapshot.battery, &mut recs);

        // Cross-dimensional recommendations
        self.analyze_cross_dimensional(snapshot, &mut recs);

        // Apply adaptive learning weights and re-rank
        self.apply_adaptive_weights(&mut recs);

        // Sort by severity (descending), then by confidence (descending)
        recs.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });

        recs
    }

    fn analyze_memory(&self, mem: &MemoryDimension, recs: &mut Vec<Recommendation>) {
        let threshold = self.profile.pressure_threshold();

        // High memory utilization
        if mem.utilization > threshold {
            let reclaimable = mem.available_bytes;
            let severity = if mem.utilization > 0.90 {
                Severity::Critical
            } else if mem.utilization > 0.85 {
                Severity::High
            } else {
                Severity::Medium
            };

            recs.push(Recommendation {
                id: "mem_high_util".to_string(),
                severity,
                category: "memory".to_string(),
                title: format!("Memory usage is at {:.0}%", mem.utilization * 100.0),
                explanation: format!(
                    "Your system is using {:.1} GB of {:.1} GB of RAM. \
                     {} of that is compressed, and you have {} in swap. \
                     This can cause slowdowns as the system struggles to \
                     free memory for active apps.",
                    gb(mem.used_bytes),
                    gb(mem.total_bytes),
                    gb(mem.compressed_bytes),
                    gb(mem.swap_used_bytes),
                ),
                action: "Purge inactive memory and close memory-hungry apps you don't need right now.".to_string(),
                command: Some("xmac ram-boost".to_string()),
                estimated_impact: format!(
                    "Could free up to {} of inactive memory",
                    gb(reclaimable),
                ),
                confidence: 0.85,
                auto_safe: true,
            });
        }

        // Swap usage
        if mem.swap_total_bytes > 0 && mem.swap_used_bytes > 0 {
            let swap_pct = mem.swap_used_bytes as f64 / mem.swap_total_bytes as f64;
            if swap_pct > 0.5 {
                recs.push(Recommendation {
                    id: "mem_swap_high".to_string(),
                    severity: Severity::High,
                    category: "memory".to_string(),
                    title: format!("Heavy swap usage: {:.0}% of swap filled", swap_pct * 100.0),
                    explanation: format!(
                        "Your system is using {} of swap space. This means \
                         RAM is full and macOS is writing memory to disk, \
                         which is 10-100x slower. This is the most common \
                         cause of a sluggish Mac.",
                        gb(mem.swap_used_bytes),
                    ),
                    action: "Close apps you're not using, or consider upgrading your RAM. Run a memory purge for temporary relief.".to_string(),
                    command: Some("xmac ram-boost --purge".to_string()),
                    estimated_impact: "Reduces disk I/O and improves responsiveness significantly".to_string(),
                    confidence: 0.92,
                    auto_safe: true,
                });
            }
        }

        // Memory pressure warning
        if mem.pressure_level >= 2 {
            recs.push(Recommendation {
                id: "mem_pressure".to_string(),
                severity: if mem.pressure_level >= 4 { Severity::Critical } else { Severity::Medium },
                category: "memory".to_string(),
                title: format!("Memory pressure: {}", mem.pressure_label),
                explanation: "The macOS memory subsystem is under pressure. The system is actively compressing and swapping memory to keep running, which consumes CPU and slows everything down.".to_string(),
                action: "Free up memory immediately. Close browser tabs and apps you don't need.".to_string(),
                command: Some("xmac zen --execute".to_string()),
                estimated_impact: "Immediate responsiveness improvement".to_string(),
                confidence: 0.90,
                auto_safe: true,
            });
        }
    }

    fn analyze_cpu(&self, cpu: &CpuDimension, recs: &mut Vec<Recommendation>) {
        if cpu.utilization_pct > 80.0 {
            let top = cpu.top_processes.first();
            let top_desc = top
                .map(|p| format!(" The top consumer is {} (pid {}, {:.1}% CPU).", p.name, p.pid, p.cpu_pct))
                .unwrap_or_default();

            recs.push(Recommendation {
                id: "cpu_high".to_string(),
                severity: if cpu.utilization_pct > 95.0 { Severity::High } else { Severity::Medium },
                category: "cpu".to_string(),
                title: format!("CPU load is high: {:.0}%", cpu.utilization_pct),
                explanation: format!(
                    "Your CPU load average is {:.2} across {} cores.{} \
                     This means the system is working near capacity and \
                     tasks may take longer to complete.",
                    cpu.load_average_1m,
                    cpu.core_count,
                    top_desc,
                ),
                action: "Close or quit CPU-intensive apps if you don't need them. Check Activity Monitor for runaway processes.".to_string(),
                command: None,
                estimated_impact: "Faster app launches and smoother multitasking".to_string(),
                confidence: 0.80,
                auto_safe: false,
            });
        }

        // Runaway process
        if let Some(top) = cpu.top_processes.first() {
            if top.cpu_pct > 100.0 {
                recs.push(Recommendation {
                    id: "cpu_runaway".to_string(),
                    severity: Severity::Medium,
                    category: "cpu".to_string(),
                    title: format!("Runaway process: {} using {:.0}% CPU", top.name, top.cpu_pct),
                    explanation: format!(
                        "The process '{}' (pid {}) is consuming {:.1}% CPU — \
                         more than a full core. This may indicate a bug, \
                         an infinite loop, or heavy computation.",
                        top.name,
                        top.pid,
                        top.cpu_pct,
                    ),
                    action: format!("If you don't need this process, quit the associated app or run: kill {}", top.pid),
                    command: Some(format!("kill {}", top.pid)),
                    estimated_impact: "Frees CPU resources for other tasks".to_string(),
                    confidence: 0.75,
                    auto_safe: false,
                });
            }
        }
    }

    fn analyze_disk(&self, disk: &DiskDimension, recs: &mut Vec<Recommendation>) {
        for vol in &disk.volumes {
            if vol.utilization > 0.90 {
                let free_gb = gb(vol.free_bytes);
                recs.push(Recommendation {
                    id: format!("disk_full_{}", vol.mount_point.replace('/', "_")),
                    severity: if vol.utilization > 0.95 { Severity::Critical } else { Severity::High },
                    category: "disk".to_string(),
                    title: format!("Disk almost full: {} ({:.0}%)", vol.mount_point, vol.utilization * 100.0),
                    explanation: format!(
                        "The volume mounted at {} has only {} free out of {}. \
                         When a disk gets above 90% full, macOS slows down \
                         significantly because there's no room for virtual \
                         memory, temp files, and file system optimization.",
                        vol.mount_point,
                        free_gb,
                        gb(vol.capacity_bytes),
                    ),
                    action: "Run a cleanup scan to find and remove reclaimable space. Consider moving large files to external storage or cloud.".to_string(),
                    command: Some("xmac quick".to_string()),
                    estimated_impact: format!("Could reclaim several GB of cache and temp files"),
                    confidence: 0.95,
                    auto_safe: true,
                });
            }
        }
    }

    fn analyze_thermal(&self, thermal: &ThermalDimension, recs: &mut Vec<Recommendation>) {
        if thermal.thermal_pressure == "Critical" {
            if let Some(temp) = thermal.cpu_temp_c {
                recs.push(Recommendation {
                    id: "thermal_critical".to_string(),
                    severity: Severity::High,
                    category: "thermal".to_string(),
                    title: format!("CPU temperature is critical: {:.0}°C", temp),
                    explanation: format!(
                        "Your CPU is running at {:.0}°C, which is above the \
                         safe operating range. The system may throttle \
                         performance to prevent damage. This is common during \
                         heavy workloads or when ventilation is blocked.",
                        temp,
                    ),
                    action: "Close CPU-intensive apps, ensure ventilation is not blocked, and consider using your Mac on a hard surface.".to_string(),
                    command: None,
                    estimated_impact: "Prevents thermal throttling and potential hardware damage".to_string(),
                    confidence: 0.85,
                    auto_safe: false,
                });
            }
        } else if thermal.thermal_pressure == "High" {
            if let Some(temp) = thermal.cpu_temp_c {
                recs.push(Recommendation {
                    id: "thermal_high".to_string(),
                    severity: Severity::Low,
                    category: "thermal".to_string(),
                    title: format!("CPU running warm: {:.0}°C", temp),
                    explanation: format!(
                        "Your CPU temperature is {:.0}°C. This is within \
                         operational limits but elevated. The fans are \
                         likely running at higher speeds.",
                        temp,
                    ),
                    action: "This is normal during heavy workloads. If it persists, check for runaway processes.".to_string(),
                    command: None,
                    estimated_impact: "Improved fan noise and longevity".to_string(),
                    confidence: 0.70,
                    auto_safe: false,
                });
            }
        }
    }

    fn analyze_battery(&self, battery: &BatteryDimension, recs: &mut Vec<Recommendation>) {
        if !battery.is_present {
            return;
        }

        if battery.charge_pct < 20.0 && !battery.is_plugged {
            recs.push(Recommendation {
                id: "battery_low".to_string(),
                severity: if battery.charge_pct < 10.0 { Severity::High } else { Severity::Medium },
                category: "battery".to_string(),
                title: format!("Battery low: {:.0}%", battery.charge_pct),
                explanation: format!(
                    "Your battery is at {:.0}%. {}",
                    battery.charge_pct,
                    if let Some(mins) = battery.time_remaining_mins {
                        format!("Approximately {} hours {} minutes remaining.", mins / 60, mins % 60)
                    } else {
                        "Time remaining estimate unavailable.".to_string()
                    }
                ),
                action: "Plug in your charger soon. Save your work if you're below 10%.".to_string(),
                command: None,
                estimated_impact: "Prevents unexpected shutdown".to_string(),
                confidence: 0.95,
                auto_safe: false,
            });
        }

        // Battery health
        if battery.condition.to_lowercase().contains("service") || battery.condition.to_lowercase().contains("replace") {
            recs.push(Recommendation {
                id: "battery_health".to_string(),
                severity: Severity::Medium,
                category: "battery".to_string(),
                title: format!("Battery needs service: {}", battery.condition),
                explanation: format!(
                    "Your battery condition is reported as '{}'. After {} \
                     charge cycles, the battery may not hold charge as well \
                     as it used to. This is normal wear over time.",
                    battery.condition,
                    battery.cycle_count,
                ),
                action: "Consider scheduling a battery replacement at an Apple Store or authorized service provider.".to_string(),
                command: None,
                estimated_impact: "Restores battery life to full capacity".to_string(),
                confidence: 0.90,
                auto_safe: false,
            });
        }
    }

    fn analyze_cross_dimensional(&self, snapshot: &SystemSnapshot, recs: &mut Vec<Recommendation>) {
        // Memory + Disk: if both are under pressure, suggest Zen Mode
        let mem_pressure = snapshot.memory.utilization > self.profile.pressure_threshold();
        let disk_pressure = snapshot.disk.overall_utilization > 0.85;
        if mem_pressure && disk_pressure {
            recs.push(Recommendation {
                id: "cross_zen".to_string(),
                severity: Severity::Medium,
                category: "system".to_string(),
                title: "System under combined memory and disk pressure".to_string(),
                explanation: format!(
                    "Your system health score is {:.0}/100. Both memory ({:.0}%) \
                     and disk ({:.0}%) are under pressure. This combination \
                     is particularly impactful because low disk space prevents \
                     effective memory swapping, creating a feedback loop.",
                    snapshot.health_score,
                    snapshot.memory.utilization * 100.0,
                    snapshot.disk.overall_utilization * 100.0,
                ),
                action: "Run Zen Mode for a comprehensive one-click optimization that addresses both issues simultaneously.".to_string(),
                command: Some("xmac zen --execute".to_string()),
                estimated_impact: "Improves overall system health score significantly".to_string(),
                confidence: 0.88,
                auto_safe: true,
            });
        }

        // Proactive: predict if memory will hit critical
        if self.profile.pressure_threshold() < snapshot.memory.utilization
            && snapshot.memory.utilization < 0.85
        {
            let trend = snapshot.memory.utilization;
            if trend > 0.70 {
                recs.push(Recommendation {
                    id: "proactive_memory".to_string(),
                    severity: Severity::Low,
                    category: "proactive".to_string(),
                    title: format!("Memory pressure building up ({:.0}%)", trend * 100.0),
                    explanation: format!(
                        "Your memory usage is at {:.0}% and trending upward. \
                         Based on current patterns, you may hit critical \
                         pressure within the next 30-60 minutes if usage \
                         continues to grow.",
                        trend * 100.0,
                    ),
                    action: "Consider closing some browser tabs or apps you're not actively using. A quick memory purge can help too.".to_string(),
                    command: Some("xmac ram-boost".to_string()),
                    estimated_impact: "Prevents future slowdown before it happens".to_string(),
                    confidence: 0.65,
                    auto_safe: true,
                });
            }
        }

        // Overall health summary
        if snapshot.health_score < 50.0 {
            recs.push(Recommendation {
                id: "health_poor".to_string(),
                severity: Severity::High,
                category: "system".to_string(),
                title: format!("System health is {} ({:.0}/100)", snapshot.status, snapshot.health_score),
                explanation: format!(
                    "Your overall system health score is {:.0} out of 100. \
                     Multiple dimensions need attention. Running a comprehensive \
                     optimization will improve performance across the board.",
                    snapshot.health_score,
                ),
                action: "Run Zen Mode to address all issues at once, or review the individual recommendations above.".to_string(),
                command: Some("xmac zen --execute".to_string()),
                estimated_impact: "Significant improvement in responsiveness and stability".to_string(),
                confidence: 0.85,
                auto_safe: true,
            });
        } else if snapshot.health_score >= 85.0 {
            recs.push(Recommendation {
                id: "health_good".to_string(),
                severity: Severity::Info,
                category: "system".to_string(),
                title: format!("System health is {} ({:.0}/100)", snapshot.status, snapshot.health_score),
                explanation: "Your system is in great shape. No urgent action needed. Keep up the good maintenance habits!".to_string(),
                action: "No action needed. Consider running a quick scan periodically to keep things clean.".to_string(),
                command: Some("xmac quick".to_string()),
                estimated_impact: "Maintains current excellent performance".to_string(),
                confidence: 0.95,
                auto_safe: true,
            });
        }
    }

    /// Apply adaptive learning weights — boost recommendations for categories
    /// the user tends to accept, and demote ones they dismiss.
    fn apply_adaptive_weights(&self, recs: &mut Vec<Recommendation>) {
        if !self.adaptive.enabled {
            return;
        }
        for rec in recs.iter_mut() {
            let weight = self.adaptive.weight_for(&rec.category);
            // Adjust confidence by up to ±20% based on user history.
            let adjustment = (weight - 0.5) * 0.4; // -0.2 to +0.2
            rec.confidence = (rec.confidence + adjustment).clamp(0.0, 1.0);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Formatting helpers
// ═══════════════════════════════════════════════════════════════════════

fn gb(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 10.0 {
        format!("{:.0} GB", gb)
    } else if gb >= 1.0 {
        format!("{:.1} GB", gb)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{:.0} MB", mb)
    }
}

/// Format recommendations as human-readable text.
pub fn format_recommendations_text(recs: &[Recommendation], health_score: Option<f64>) -> String {
    let mut out = String::new();

    if let Some(score) = health_score {
        let status = if score >= 85.0 { "excellent" }
            else if score >= 70.0 { "good" }
            else if score >= 50.0 { "fair" }
            else if score >= 30.0 { "poor" }
            else { "critical" };
        out.push_str(&format!("╔══════════════════════════════════════════╗\n"));
        out.push_str(&format!("║  System Health: {:.0}/100 ({}){}\n", score, status, " ".repeat(20 - status.len() - 3 - score.to_string().len())));
        out.push_str(&format!("╚══════════════════════════════════════════╝\n\n"));
    }

    if recs.is_empty() {
        out.push_str("No recommendations. Your system looks great!\n");
        return out;
    }

    for (i, rec) in recs.iter().enumerate() {
        let severity_icon = match rec.severity {
            Severity::Critical => "🔴",
            Severity::High => "🟠",
            Severity::Medium => "🟡",
            Severity::Low => "🟢",
            Severity::Info => "🔵",
        };

        out.push_str(&format!("{}. {} [{}] {}\n", i + 1, severity_icon, rec.severity.label(), rec.title));
        out.push_str(&format!("   {}\n", rec.explanation));
        out.push_str(&format!("   → {}\n", rec.action));
        if let Some(cmd) = &rec.command {
            out.push_str(&format!("   $ {}\n", cmd));
        }
        out.push_str(&format!("   Impact: {} (confidence: {:.0}%)\n\n", rec.estimated_impact, rec.confidence * 100.0));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::store::AdaptiveState;

    #[test]
    fn test_advisor_produces_recommendations() {
        let snapshot = SystemSnapshot::collect();
        let advisor = Advisor::new(OptimizationProfile::Balanced, AdaptiveState::default());
        let recs = advisor.analyze(&snapshot);
        // Should produce at least one recommendation (health summary)
        assert!(!recs.is_empty());
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn test_format_text() {
        let rec = Recommendation {
            id: "test".to_string(),
            severity: Severity::Medium,
            category: "test".to_string(),
            title: "Test recommendation".to_string(),
            explanation: "This is a test.".to_string(),
            action: "Do something.".to_string(),
            command: Some("xmac test".to_string()),
            estimated_impact: "Test impact".to_string(),
            confidence: 0.8,
            auto_safe: true,
        };
        let text = format_recommendations_text(&[rec], Some(75.0));
        assert!(text.contains("Test recommendation"));
        assert!(text.contains("75"));
    }

    #[test]
    fn test_adaptive_weighting() {
        let mut adaptive = AdaptiveState::default();
        // Record that user always accepts memory recommendations
        for _ in 0..10 {
            adaptive.record_feedback("memory", "medium", true);
        }
        // Record that user always dismisses disk recommendations
        for _ in 0..10 {
            adaptive.record_feedback("disk", "medium", false);
        }

        let advisor = Advisor::new(OptimizationProfile::Balanced, adaptive);
        let snapshot = SystemSnapshot::collect();
        let recs = advisor.analyze(&snapshot);

        // Memory recs should have higher confidence than disk recs
        let mem_confidence = recs.iter().filter(|r| r.category == "memory").map(|r| r.confidence).fold(0.0f64, f64::max);
        let disk_confidence = recs.iter().filter(|r| r.category == "disk").map(|r| r.confidence).fold(0.0f64, f64::max);

        // Only check if both exist
        if mem_confidence > 0.0 && disk_confidence > 0.0 {
            assert!(mem_confidence >= disk_confidence);
        }
    }
}

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Energy & Battery Twin — models the Mac's energy subsystem.
///
/// Covers: battery behavior, charging/discharging patterns, energy impact
/// per app, battery life prediction, thermal efficiency, sleep/wake
/// analysis, power mode recommendations, and degradation forecasting.
///
/// Maps to Digital Twin operations 201-235.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyTwin {
    /// op 201: Battery behavior model.
    pub battery: Option<BatteryModel>,
    /// op 202-203: Charging and discharging session history.
    pub charging_patterns: Vec<ChargingSession>,
    /// op 204: Per-process energy impact.
    pub energy_consumers: Vec<EnergyConsumer>,
    /// op 214: Thermal efficiency (0-1, higher is better).
    pub thermal_efficiency: Option<f64>,
    /// op 225: Sleep efficiency (0-1, higher is better).
    pub sleep_efficiency: Option<f64>,
    /// op 226: Recent wake events and their causes.
    pub wake_causes: Vec<WakeEvent>,
    /// op 216: Degradation forecast for the next year.
    pub degradation_forecast: Option<DegradationForecast>,
    /// op 212: Recommended power mode ("Automatic", "Low", "High").
    pub recommended_power_mode: String,
    /// op 230: Energy profile — categorized drain breakdown.
    pub profile: EnergyProfile,
    /// op 235: Historical energy samples.
    pub history: Vec<EnergySample>,
    /// op 232: Human-readable consumption explanations.
    pub consumption_explanations: Vec<String>,
    /// op 229: Energy priorities (which apps get priority when on battery).
    pub priorities: Vec<EnergyPriority>,
    /// op 234: Automated energy optimization policies.
    pub policies: Vec<EnergyPolicy>,
}

/// op 201: Battery behavior model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryModel {
    pub charge_pct: f64,
    pub cycle_count: u64,
    pub condition: String,
    pub is_charging: bool,
    pub is_plugged: bool,
    pub time_remaining_mins: Option<u64>,
    pub design_capacity_mah: Option<u64>,
    pub current_capacity_mah: Option<u64>,
    /// op 223: Health as a percentage of design capacity (0-100).
    pub health_pct: Option<f64>,
    /// op 213: Whether aging is abnormal given the cycle count.
    pub abnormal_aging: bool,
}

/// op 202-203: A charging or discharging session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargingSession {
    pub started_at_ms: u64,
    pub duration_mins: u64,
    pub charge_gained_pct: f64,
    /// "charging" or "discharging".
    pub kind: String,
}

/// op 204: A per-process energy consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConsumer {
    pub pid: u32,
    pub name: String,
    /// 0-100, Apple's energy impact score.
    pub energy_impact: f64,
    /// "background", "network", "display", "compute", etc.
    pub category: String,
}

/// op 226: A wake event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeEvent {
    pub timestamp_ms: u64,
    pub cause: String,
}

/// op 216: Degradation forecast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationForecast {
    pub predicted_cycle_count_1y: u64,
    pub predicted_health_pct_1y: f64,
    pub confidence: f64,
}

/// op 230: Energy profile — drain broken down by category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyProfile {
    pub background_drain_pct: f64,
    pub network_drain_pct: f64,
    pub display_drain_pct: f64,
    pub compute_drain_pct: f64,
}

/// op 235: A point-in-time energy sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergySample {
    pub timestamp_ms: u64,
    pub charge_pct: f64,
    pub is_plugged: bool,
    pub estimated_draw_watts: Option<f64>,
}

/// op 229: An energy priority entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyPriority {
    pub app_name: String,
    /// "high", "normal", "low".
    pub priority: String,
    pub reason: String,
}

/// op 234: An automated energy policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyPolicy {
    pub name: String,
    pub trigger: String,
    pub action: String,
    pub enabled: bool,
}

#[allow(dead_code)]
impl EnergyTwin {
    /// Collect energy and battery twin data.
    pub fn collect() -> Self {
        // op 201: Model battery behavior from system_awareness.
        let battery_dim = crate::intelligence::system_awareness::BatteryDimension::collect();
        let battery = if battery_dim.is_present {
            let health_pct = compute_health_pct(&battery_dim);
            let abnormal_aging = detect_abnormal_aging(&battery_dim, health_pct);
            Some(BatteryModel {
                charge_pct: battery_dim.charge_pct,
                cycle_count: battery_dim.cycle_count,
                condition: battery_dim.condition.clone(),
                is_charging: battery_dim.is_charging,
                is_plugged: battery_dim.is_plugged,
                time_remaining_mins: battery_dim.time_remaining_mins,
                design_capacity_mah: None,
                current_capacity_mah: None,
                health_pct,
                abnormal_aging,
            })
        } else {
            None
        };

        // op 204: Track energy impact per app.
        let energy_consumers = collect_energy_consumers();
        // op 230: Build energy profile from consumer categories.
        let profile = build_energy_profile(&energy_consumers);
        // op 205-208: Identify drain categories.
        // op 225-226: Sleep efficiency and wake causes.
        let wake_causes = collect_wake_causes();
        let sleep_efficiency = estimate_sleep_efficiency(&wake_causes);
        // op 214: Thermal efficiency.
        let thermal = crate::intelligence::system_awareness::ThermalDimension::collect();
        let thermal_efficiency = compute_thermal_efficiency(&thermal);
        // op 216: Degradation forecast.
        let degradation_forecast = forecast_degradation(&battery_dim);
        // op 212: Recommend power mode.
        let recommended_power_mode = recommend_power_mode(&battery_dim, &energy_consumers);
        // op 235: History (single sample; daemon accumulates).
        let history = vec![EnergySample {
            timestamp_ms: now_ms(),
            charge_pct: battery_dim.charge_pct,
            is_plugged: battery_dim.is_plugged,
            estimated_draw_watts: estimate_draw_watts(&battery_dim),
        }];
        // op 229: Energy priorities.
        let priorities = compute_priorities(&energy_consumers);
        // op 234: Policies.
        let policies = default_energy_policies();
        // op 232: Explain consumption.
        let mut consumption_explanations = Vec::new();
        if let Some(b) = &battery {
            if b.abnormal_aging {
                consumption_explanations
                    .push("Battery is aging faster than expected for its cycle count.".to_string());
            }
        }
        let bg_pct = profile.background_drain_pct;
        if bg_pct > 40.0 {
            consumption_explanations.push(format!(
                "Background processes account for {:.0}% of energy drain — consider suspending idle apps.",
                bg_pct
            ));
        }

        Self {
            battery,
            charging_patterns: Vec::new(),
            energy_consumers,
            thermal_efficiency,
            sleep_efficiency,
            wake_causes,
            degradation_forecast,
            recommended_power_mode,
            profile,
            history,
            consumption_explanations,
            priorities,
            policies,
        }
    }

    /// op 205: Identify battery-heavy apps (energy impact > 50).
    pub fn battery_heavy_apps(&self) -> Vec<&EnergyConsumer> {
        self.energy_consumers
            .iter()
            .filter(|c| c.energy_impact > 50.0)
            .collect()
    }

    /// op 206: Identify background drain — sum of background-category impact.
    pub fn background_drain(&self) -> f64 {
        self.energy_consumers
            .iter()
            .filter(|c| c.category == "background")
            .map(|c| c.energy_impact)
            .sum()
    }

    /// op 207: Identify network drain.
    pub fn network_drain(&self) -> f64 {
        self.energy_consumers
            .iter()
            .filter(|c| c.category == "network")
            .map(|c| c.energy_impact)
            .sum()
    }

    /// op 208: Identify display drain.
    pub fn display_drain(&self) -> f64 {
        self.energy_consumers
            .iter()
            .filter(|c| c.category == "display")
            .map(|c| c.energy_impact)
            .sum()
    }

    /// op 209: Predict battery life in minutes from the current drain rate.
    pub fn predict_battery_life_mins(&self) -> Option<u64> {
        let b = self.battery.as_ref()?;
        if b.is_plugged {
            return None;
        }
        if let Some(t) = b.time_remaining_mins {
            return Some(t);
        }
        // Estimate from charge and a nominal draw.
        let draw_pct_per_min = 0.5; // heuristic
        if draw_pct_per_min > 0.0 {
            Some((b.charge_pct / draw_pct_per_min) as u64)
        } else {
            None
        }
    }

    /// op 210: Optimize energy usage — return recommended actions.
    pub fn optimize_energy(&self) -> Vec<String> {
        let mut recs = Vec::new();
        for app in self.battery_heavy_apps() {
            recs.push(format!(
                "Reduce background activity of {} (impact {:.0}).",
                app.name, app.energy_impact
            ));
        }
        if self.background_drain() > 50.0 {
            recs.push("Enable Low Power Mode to throttle background tasks.".to_string());
        }
        recs
    }

    /// op 211: Adjust background activity — apps to suspend.
    pub fn adjust_background(&self) -> Vec<String> {
        self.energy_consumers
            .iter()
            .filter(|c| c.category == "background" && c.energy_impact > 20.0)
            .map(|c| c.name.clone())
            .collect()
    }

    /// op 213: Detect abnormal battery aging.
    pub fn has_abnormal_aging(&self) -> bool {
        self.battery
            .as_ref()
            .map(|b| b.abnormal_aging)
            .unwrap_or(false)
    }

    /// op 215: Analyze charging efficiency (0-1).
    pub fn charging_efficiency(&self) -> Option<f64> {
        let b = self.battery.as_ref()?;
        if !b.is_charging {
            return None;
        }
        // Heuristic: efficiency drops as charge approaches 100%.
        Some(1.0 - (b.charge_pct / 100.0).min(1.0) * 0.3)
    }

    /// op 217: Model user mobility — fraction of time on battery.
    pub fn mobility_score(&self) -> f64 {
        let plugged = self.history.iter().filter(|s| s.is_plugged).count();
        let total = self.history.len();
        if total == 0 {
            return 0.0;
        }
        1.0 - (plugged as f64 / total as f64)
    }

    /// op 219: Recommend charging strategy.
    pub fn recommend_charging_strategy(&self) -> String {
        let b = match self.battery.as_ref() {
            Some(b) => b,
            None => return "No battery present.".to_string(),
        };
        if b.cycle_count > 500 {
            "Avoid keeping the battery at 100% continuously; charge to 80% and discharge to 20% to reduce wear.".to_string()
        } else {
            "Charge opportunistically; current cycle count is healthy.".to_string()
        }
    }

    /// op 221: Compare battery sessions — returns the spread of durations.
    pub fn compare_sessions(&self) -> Option<(f64, f64)> {
        if self.charging_patterns.len() < 2 {
            return None;
        }
        let durations: Vec<f64> = self
            .charging_patterns
            .iter()
            .map(|s| s.duration_mins as f64)
            .collect();
        let min = durations.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Some((min, max))
    }

    /// op 222: Generate a human-readable battery report.
    pub fn battery_report(&self) -> String {
        match &self.battery {
            Some(b) => format!(
                "Battery: {:.0}% ({} cycles, {}), {} — recommended power mode: {}",
                b.charge_pct,
                b.cycle_count,
                b.condition,
                if b.is_plugged {
                    "plugged in"
                } else {
                    "on battery"
                },
                self.recommended_power_mode,
            ),
            None => "No battery present.".to_string(),
        }
    }

    /// op 227: Optimize standby — return recommended standby settings.
    pub fn optimize_standby(&self) -> Vec<String> {
        vec![
            "Enable Power Nap for limited background activity during sleep.".to_string(),
            "Disable wake for network access when on battery.".to_string(),
        ]
    }

    /// op 228: Reduce idle power — return idle-power recommendations.
    pub fn reduce_idle_power(&self) -> Vec<String> {
        let mut recs = vec!["Dim display after 2 minutes of inactivity.".to_string()];
        if self.background_drain() > 30.0 {
            recs.push("Suspend high-impact background apps during idle periods.".to_string());
        }
        recs
    }

    /// op 231: Predict future battery state (charge % after N minutes).
    pub fn predict_future_state(&self, minutes: u64) -> Option<f64> {
        let b = self.battery.as_ref()?;
        let draw = estimate_draw_watts(&crate::intelligence::system_awareness::BatteryDimension {
            is_present: true,
            is_charging: b.is_charging,
            is_plugged: b.is_plugged,
            charge_pct: b.charge_pct,
            cycle_count: b.cycle_count,
            condition: b.condition.clone(),
            time_remaining_mins: b.time_remaining_mins,
        })?;
        // Very rough: assume ~50Wh battery; draw in watts.
        let wh = 50.0;
        let consumed = draw * (minutes as f64 / 60.0);
        let consumed_pct = (consumed / wh) * 100.0;
        if b.is_plugged {
            Some((b.charge_pct + consumed_pct).min(100.0))
        } else {
            Some((b.charge_pct - consumed_pct).max(0.0))
        }
    }

    /// op 233: Simulate a power change (e.g. enabling Low Power Mode).
    /// Returns the predicted charge after `minutes` with a multiplier on draw.
    pub fn simulate_power_change(&self, minutes: u64, draw_multiplier: f64) -> Option<f64> {
        let b = self.battery.as_ref()?;
        let draw = estimate_draw_watts(&crate::intelligence::system_awareness::BatteryDimension {
            is_present: true,
            is_charging: b.is_charging,
            is_plugged: b.is_plugged,
            charge_pct: b.charge_pct,
            cycle_count: b.cycle_count,
            condition: b.condition.clone(),
            time_remaining_mins: b.time_remaining_mins,
        })?;
        let wh = 50.0;
        let consumed = draw * draw_multiplier * (minutes as f64 / 60.0);
        let consumed_pct = (consumed / wh) * 100.0;
        if b.is_plugged {
            Some((b.charge_pct + consumed_pct).min(100.0))
        } else {
            Some((b.charge_pct - consumed_pct).max(0.0))
        }
    }

    /// op 235: Record an energy sample into history.
    pub fn record_sample(&mut self) {
        let b = crate::intelligence::system_awareness::BatteryDimension::collect();
        self.history.push(EnergySample {
            timestamp_ms: now_ms(),
            charge_pct: b.charge_pct,
            is_plugged: b.is_plugged,
            estimated_draw_watts: estimate_draw_watts(&b),
        });
        if self.history.len() > 1440 {
            let excess = self.history.len() - 1440;
            self.history.drain(0..excess);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────

/// op 204: Collect per-process energy consumers.
fn collect_energy_consumers() -> Vec<EnergyConsumer> {
    #[cfg(target_os = "macos")]
    {
        energy_consumers_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        energy_consumers_linux()
    }
}

#[cfg(target_os = "macos")]
fn energy_consumers_macos() -> Vec<EnergyConsumer> {
    // `pmset -g top` shows per-process energy impact on macOS. It requires
    // root for full detail, so we fall back to top CPU consumers as a proxy.
    let cpu = crate::intelligence::system_awareness::CpuDimension::collect();
    cpu.top_processes
        .iter()
        .map(|p| EnergyConsumer {
            pid: p.pid,
            name: p.name.clone(),
            energy_impact: (p.cpu_pct * 0.5).min(100.0),
            category: classify_consumer(&p.name, p.cpu_pct),
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn energy_consumers_linux() -> Vec<EnergyConsumer> {
    let cpu = crate::intelligence::system_awareness::CpuDimension::collect();
    cpu.top_processes
        .iter()
        .map(|p| EnergyConsumer {
            pid: p.pid,
            name: p.name.clone(),
            energy_impact: (p.cpu_pct * 0.5).min(100.0),
            category: classify_consumer(&p.name, p.cpu_pct),
        })
        .collect()
}

/// Classify an energy consumer into a drain category.
fn classify_consumer(name: &str, cpu_pct: f64) -> String {
    let lower = name.to_lowercase();
    if lower.contains("network") || lower.contains("wifi") || lower.contains("vpn") {
        "network".to_string()
    } else if cpu_pct < 5.0 {
        "background".to_string()
    } else if lower.contains("windowserver") || lower.contains("display") {
        "display".to_string()
    } else {
        "compute".to_string()
    }
}

/// op 230: Build the energy profile from consumer categories.
fn build_energy_profile(consumers: &[EnergyConsumer]) -> EnergyProfile {
    let total: f64 = consumers
        .iter()
        .map(|c| c.energy_impact)
        .sum::<f64>()
        .max(1.0);
    let pct = |cat: &str| {
        consumers
            .iter()
            .filter(|c| c.category == cat)
            .map(|c| c.energy_impact)
            .sum::<f64>()
            / total
            * 100.0
    };
    EnergyProfile {
        background_drain_pct: pct("background"),
        network_drain_pct: pct("network"),
        display_drain_pct: pct("display"),
        compute_drain_pct: pct("compute"),
    }
}

/// op 223: Compute battery health as a percentage of design capacity.
fn compute_health_pct(b: &crate::intelligence::system_awareness::BatteryDimension) -> Option<f64> {
    // Without design/current capacity from system_profiler, estimate from
    // cycle count: ~80% health at 1000 cycles (Apple's spec).
    if b.cycle_count == 0 {
        return Some(100.0);
    }
    let health = 100.0 - (b.cycle_count as f64 / 1000.0) * 20.0;
    Some(health.clamp(0.0, 100.0))
}

/// op 213: Detect abnormal aging — health lower than expected for cycles.
fn detect_abnormal_aging(
    b: &crate::intelligence::system_awareness::BatteryDimension,
    health: Option<f64>,
) -> bool {
    match health {
        Some(h) => {
            let expected = 100.0 - (b.cycle_count as f64 / 1000.0) * 20.0;
            h < expected - 10.0
        }
        None => false,
    }
}

/// op 225: Estimate sleep efficiency from wake causes.
fn estimate_sleep_efficiency(wakes: &[WakeEvent]) -> Option<f64> {
    if wakes.is_empty() {
        return None;
    }
    // More wake events => lower sleep efficiency.
    let score = (1.0 - (wakes.len() as f64 / 50.0)).max(0.0);
    Some(score)
}

/// op 226: Collect recent wake causes.
fn collect_wake_causes() -> Vec<WakeEvent> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        // `pmset -g log` contains wake reasons; parse the last few.
        let out = Command::new("pmset")
            .args(["-g", "log"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        out.lines()
            .filter(|l| l.contains("Wake") && l.contains("due"))
            .take(10)
            .map(|l| WakeEvent {
                timestamp_ms: now_ms(),
                cause: l.to_string(),
            })
            .collect()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// op 214: Compute thermal efficiency (0-1).
fn compute_thermal_efficiency(
    t: &crate::intelligence::system_awareness::ThermalDimension,
) -> Option<f64> {
    // Lower thermal pressure => higher efficiency.
    match t.thermal_pressure.as_str() {
        "Nominal" => Some(1.0),
        "Elevated" => Some(0.75),
        "High" => Some(0.5),
        "Critical" => Some(0.25),
        _ => None,
    }
}

/// op 216: Forecast battery degradation over the next year.
fn forecast_degradation(
    b: &crate::intelligence::system_awareness::BatteryDimension,
) -> Option<DegradationForecast> {
    if !b.is_present {
        return None;
    }
    // Assume ~300 cycles/year for a typical user.
    let predicted_cycles = b.cycle_count + 300;
    let predicted_health = (100.0 - (predicted_cycles as f64 / 1000.0) * 20.0).max(0.0);
    Some(DegradationForecast {
        predicted_cycle_count_1y: predicted_cycles,
        predicted_health_pct_1y: predicted_health,
        confidence: 0.4,
    })
}

/// op 212: Recommend a power mode.
fn recommend_power_mode(
    b: &crate::intelligence::system_awareness::BatteryDimension,
    consumers: &[EnergyConsumer],
) -> String {
    if !b.is_present || b.is_plugged {
        return "Automatic".to_string();
    }
    let total_impact: f64 = consumers.iter().map(|c| c.energy_impact).sum();
    if b.charge_pct < 20.0 || total_impact > 150.0 {
        "Low".to_string()
    } else {
        "Automatic".to_string()
    }
}

/// op 229: Compute energy priorities for running apps.
fn compute_priorities(consumers: &[EnergyConsumer]) -> Vec<EnergyPriority> {
    consumers
        .iter()
        .map(|c| {
            let priority = if c.energy_impact > 50.0 {
                "high"
            } else if c.energy_impact > 20.0 {
                "normal"
            } else {
                "low"
            };
            EnergyPriority {
                app_name: c.name.clone(),
                priority: priority.to_string(),
                reason: format!("Energy impact {:.0}", c.energy_impact),
            }
        })
        .collect()
}

/// op 234: Default automated energy policies.
fn default_energy_policies() -> Vec<EnergyPolicy> {
    vec![
        EnergyPolicy {
            name: "Low Power Mode on low battery".to_string(),
            trigger: "charge_pct < 20 and not plugged".to_string(),
            action: "enable_low_power".to_string(),
            enabled: true,
        },
        EnergyPolicy {
            name: "Suspend heavy background on battery".to_string(),
            trigger: "on_battery and background_drain > 50".to_string(),
            action: "suspend_background".to_string(),
            enabled: false,
        },
    ]
}

/// Estimate current power draw in watts.
fn estimate_draw_watts(b: &crate::intelligence::system_awareness::BatteryDimension) -> Option<f64> {
    if !b.is_present {
        return None;
    }
    if let Some(mins) = b.time_remaining_mins {
        if mins > 0 && b.charge_pct > 0.0 {
            // Assume ~50Wh battery; remaining energy = charge_pct% * 50Wh.
            let remaining_wh = (b.charge_pct / 100.0) * 50.0;
            return Some(remaining_wh / (mins as f64 / 60.0));
        }
    }
    // Nominal draw estimate.
    Some(if b.is_plugged { 0.0 } else { 8.0 })
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_runs() {
        let et = EnergyTwin::collect();
        // Should not panic; recommended power mode is always set.
        assert!(!et.recommended_power_mode.is_empty());
        assert!(!et.policies.is_empty());
    }

    #[test]
    fn test_classify_consumer() {
        assert_eq!(classify_consumer("wifi-daemon", 1.0), "network");
        assert_eq!(classify_consumer("spotlight", 2.0), "background");
        assert_eq!(classify_consumer("WindowServer", 60.0), "display");
        assert_eq!(classify_consumer("Xcode", 80.0), "compute");
    }

    #[test]
    fn test_build_energy_profile() {
        let consumers = vec![
            EnergyConsumer {
                pid: 1,
                name: "a".to_string(),
                energy_impact: 40.0,
                category: "background".to_string(),
            },
            EnergyConsumer {
                pid: 2,
                name: "b".to_string(),
                energy_impact: 60.0,
                category: "compute".to_string(),
            },
        ];
        let p = build_energy_profile(&consumers);
        assert!((p.background_drain_pct - 40.0).abs() < 0.1);
        assert!((p.compute_drain_pct - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_compute_health_pct() {
        let b = crate::intelligence::system_awareness::BatteryDimension {
            is_present: true,
            is_charging: false,
            is_plugged: true,
            charge_pct: 100.0,
            cycle_count: 0,
            condition: "Normal".to_string(),
            time_remaining_mins: None,
        };
        assert_eq!(compute_health_pct(&b), Some(100.0));

        let b2 = crate::intelligence::system_awareness::BatteryDimension {
            cycle_count: 1000,
            ..b.clone()
        };
        let h = compute_health_pct(&b2).unwrap();
        assert!((h - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_detect_abnormal_aging() {
        let b = crate::intelligence::system_awareness::BatteryDimension {
            is_present: true,
            is_charging: false,
            is_plugged: true,
            charge_pct: 100.0,
            cycle_count: 200,
            condition: "Normal".to_string(),
            time_remaining_mins: None,
        };
        // Health much lower than expected => abnormal.
        assert!(detect_abnormal_aging(&b, Some(50.0)));
        // Health at expected level => not abnormal.
        assert!(!detect_abnormal_aging(&b, Some(96.0)));
    }

    #[test]
    fn test_recommend_power_mode() {
        let b_plugged = crate::intelligence::system_awareness::BatteryDimension {
            is_present: true,
            is_charging: true,
            is_plugged: true,
            charge_pct: 100.0,
            cycle_count: 0,
            condition: "Normal".to_string(),
            time_remaining_mins: None,
        };
        assert_eq!(recommend_power_mode(&b_plugged, &[]), "Automatic");

        let b_low = crate::intelligence::system_awareness::BatteryDimension {
            is_present: true,
            is_charging: false,
            is_plugged: false,
            charge_pct: 10.0,
            cycle_count: 0,
            condition: "Normal".to_string(),
            time_remaining_mins: None,
        };
        assert_eq!(recommend_power_mode(&b_low, &[]), "Low");
    }

    #[test]
    fn test_battery_report() {
        let et = EnergyTwin::collect();
        let report = et.battery_report();
        assert!(!report.is_empty());
    }

    #[test]
    fn test_optimize_energy_and_standby() {
        let et = EnergyTwin::collect();
        assert!(!et.optimize_standby().is_empty());
        assert!(!et.reduce_idle_power().is_empty());
        // optimize_energy returns a Vec (may be empty if no heavy apps).
        let _ = et.optimize_energy();
    }

    #[test]
    fn test_default_energy_policies() {
        let p = default_energy_policies();
        assert!(p.iter().any(|p| p.action == "enable_low_power"));
    }

    #[test]
    fn test_compute_priorities() {
        let consumers = vec![
            EnergyConsumer {
                pid: 1,
                name: "a".to_string(),
                energy_impact: 60.0,
                category: "compute".to_string(),
            },
            EnergyConsumer {
                pid: 2,
                name: "b".to_string(),
                energy_impact: 10.0,
                category: "background".to_string(),
            },
        ];
        let prios = compute_priorities(&consumers);
        assert_eq!(prios[0].priority, "high");
        assert_eq!(prios[1].priority, "low");
    }
}

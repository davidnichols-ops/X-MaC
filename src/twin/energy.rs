use serde::{Deserialize, Serialize};

/// Energy & Battery Twin — models the Mac's energy subsystem.
///
/// Covers: battery behavior, charging/discharging patterns, energy impact
/// per app, battery life prediction, thermal efficiency, sleep/wake
/// analysis, power mode recommendations, and degradation forecasting.
///
/// Maps to Digital Twin operations 201-235.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyTwin {
    pub battery: Option<BatteryModel>,
    pub charging_patterns: Vec<ChargingSession>,
    pub energy_consumers: Vec<EnergyConsumer>,
    pub thermal_efficiency: Option<f64>,
    pub sleep_efficiency: Option<f64>,
    pub wake_causes: Vec<WakeEvent>,
    pub degradation_forecast: Option<DegradationForecast>,
    pub recommended_power_mode: String,
}

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
    pub health_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChargingSession {
    pub started_at_ms: u64,
    pub duration_mins: u64,
    pub charge_gained_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConsumer {
    pub pid: u32,
    pub name: String,
    pub energy_impact: f64,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeEvent {
    pub timestamp_ms: u64,
    pub cause: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationForecast {
    pub predicted_cycle_count_1y: u64,
    pub predicted_health_pct_1y: f64,
    pub confidence: f64,
}

impl EnergyTwin {
    /// Collect energy and battery twin data.
    pub fn collect() -> Self {
        // TODO: implement full energy twin (ops 201-235)
        // Start with what system_awareness already provides
        let battery = crate::intelligence::system_awareness::BatteryDimension::collect();

        Self {
            battery: if battery.is_present {
                Some(BatteryModel {
                    charge_pct: battery.charge_pct,
                    cycle_count: battery.cycle_count,
                    condition: battery.condition.clone(),
                    is_charging: battery.is_charging,
                    is_plugged: battery.is_plugged,
                    time_remaining_mins: battery.time_remaining_mins,
                    design_capacity_mah: None,
                    current_capacity_mah: None,
                    health_pct: None,
                })
            } else {
                None
            },
            charging_patterns: Vec::new(),
            energy_consumers: Vec::new(),
            thermal_efficiency: None,
            sleep_efficiency: None,
            wake_causes: Vec::new(),
            degradation_forecast: None,
            recommended_power_mode: "Automatic".to_string(),
        }
    }
}

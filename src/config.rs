// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::xdebug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Plugin configuration: LED dataref mappings, system datarefs, and trim wheel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_autopilot")]
    pub autopilot: AutopilotConfig,

    #[serde(default = "default_landing_gear")]
    pub landing_gear: LandingGearConfig,

    #[serde(default = "default_annunciators")]
    pub annunciators: AnnunciatorsConfig,

    #[serde(default = "default_system")]
    pub system: SystemConfig,

    #[serde(default = "default_trim_wheel")]
    pub trim_wheel: TrimWheelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotConfig {
    pub hdg: String,
    pub nav: String,
    pub apr: String,
    pub rev: String,
    pub alt: String,
    pub vs: String,
    pub ias: String,
    pub ap: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandingGearConfig {
    pub gear: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnunciatorsConfig {
    pub master_warning: String,
    pub engine_fire: String,
    pub oil_pressure_low: String,
    pub fuel_pressure_low: String,
    pub anti_ice: String,
    pub starter: String,
    pub apu: String,
    pub master_caution: String,
    pub vacuum: String,
    pub hydraulic_pressure: String,
    pub aux_fuel_pump_left: String,
    pub aux_fuel_pump_right: String,
    pub low_voltage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub bus_voltage: String,
    pub wheel_brake: String,
    pub canopy: String,
    pub doors: String,
    pub cabin_door: String,
}

/// Trim wheel configuration
///
/// The trim wheel on the Honeycomb Bravo sends 24 detent commands per full 360°
/// rotation. Each detent adjusts the elevator trim by:
///   delta = (max_trim - min_trim) / detents_per_rotation / full_turns
///
/// For a Cessna 172 (10 turns, range -1.0 to 1.0): delta = 2.0 / 24 / 10 ≈ 0.00833 per detent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrimWheelConfig {
    /// Enable trim wheel custom commands
    pub enabled: bool,
    /// Dataref to write elevator trim to
    pub elevator_trim_dataref: String,
    /// Minimum trim value (max nose down)
    pub min_trim: f32,
    /// Maximum trim value (max nose up)
    pub max_trim: f32,
    /// Number of full turns of the physical trim wheel from max nose down to max nose up
    pub full_turns: f32,
    /// Number of detents the Bravo trim wheel sends per full 360° rotation
    pub detents_per_rotation: f32,
}

impl Default for TrimWheelConfig {
    fn default() -> Self {
        TrimWheelConfig {
            enabled: true,
            elevator_trim_dataref: "sim/cockpit2/controls/elevator_trim".to_string(),
            min_trim: -1.0,
            max_trim: 1.0,
            full_turns: 10.0,
            detents_per_rotation: 24.0,
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        PluginConfig {
            autopilot: default_autopilot(),
            landing_gear: default_landing_gear(),
            annunciators: default_annunciators(),
            system: default_system(),
            trim_wheel: default_trim_wheel(),
        }
    }
}

fn default_autopilot() -> AutopilotConfig {
    AutopilotConfig {
        hdg: "sim/cockpit2/autopilot/heading_mode".to_string(),
        nav: "sim/cockpit2/autopilot/nav_status".to_string(),
        apr: "sim/cockpit2/autopilot/approach_status".to_string(),
        rev: "sim/cockpit2/autopilot/backcourse_status".to_string(),
        alt: "sim/cockpit2/autopilot/altitude_hold_status".to_string(),
        vs: "sim/cockpit2/autopilot/vvi_status".to_string(),
        ias: "sim/cockpit2/autopilot/autothrottle_on".to_string(),
        ap: "sim/cockpit2/autopilot/servos_on".to_string(),
    }
}

fn default_landing_gear() -> LandingGearConfig {
    LandingGearConfig {
        gear: "sim/flightmodel2/gear/deploy_ratio".to_string(),
    }
}

fn default_annunciators() -> AnnunciatorsConfig {
    AnnunciatorsConfig {
        master_warning: "sim/cockpit2/annunciators/master_warning".to_string(),
        engine_fire: "sim/cockpit2/annunciators/engine_fires".to_string(),
        oil_pressure_low: "sim/cockpit2/annunciators/oil_pressure_low".to_string(),
        fuel_pressure_low: "sim/cockpit2/annunciators/fuel_pressure_low".to_string(),
        anti_ice: "sim/cockpit2/annunciators/pitot_heat".to_string(),
        starter: "sim/cockpit2/engine/actuators/starter_hit".to_string(),
        apu: "sim/cockpit2/electrical/APU_running".to_string(),
        master_caution: "sim/cockpit2/annunciators/master_caution".to_string(),
        vacuum: "sim/cockpit2/annunciators/low_vacuum".to_string(),
        hydraulic_pressure: "sim/cockpit2/annunciators/hydraulic_pressure".to_string(),
        aux_fuel_pump_left: "sim/cockpit2/fuel/transfer_pump_left".to_string(),
        aux_fuel_pump_right: "sim/cockpit2/fuel/transfer_pump_right".to_string(),
        low_voltage: "sim/cockpit2/annunciators/low_voltage".to_string(),
    }
}

fn default_system() -> SystemConfig {
    SystemConfig {
        bus_voltage: "sim/cockpit2/electrical/bus_volts".to_string(),
        wheel_brake: "sim/cockpit2/controls/wheel_brake_ratio".to_string(),
        canopy: "sim/flightmodel2/misc/canopy_open_ratio".to_string(),
        doors: "sim/flightmodel2/misc/door_open_ratio".to_string(),
        cabin_door: "sim/cockpit2/annunciators/cabin_door_open".to_string(),
    }
}

fn default_trim_wheel() -> TrimWheelConfig {
    TrimWheelConfig::default()
}

/// Get the path to the X-Plane preferences directory
fn get_preferences_path() -> Option<PathBuf> {
    // Try to get X-Plane system path
    // The xplm crate doesn't expose system paths directly, so we'll use a workaround
    // We'll look for the preferences directory relative to common X-Plane locations

    // For now, use a simple approach: check if we're in the X-Plane directory structure
    // and navigate to Output/preferences
    let current_dir = std::env::current_dir().ok()?;

    // Try to find X-Plane root by looking for Resources folder
    let mut xplane_root = current_dir.clone();
    for _ in 0..5 {
        if xplane_root.join("Resources").exists() {
            let prefs_path = xplane_root.join("Output").join("preferences");
            if prefs_path.exists() || fs::create_dir_all(&prefs_path).is_ok() {
                return Some(prefs_path);
            }
        }
        if !xplane_root.pop() {
            break;
        }
    }

    // Fallback: try common X-Plane installation paths
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").ok()?;
        let paths = vec![
            PathBuf::from(&home)
                .join("X-Plane 12")
                .join("Output")
                .join("preferences"),
            PathBuf::from(&home)
                .join("Desktop")
                .join("X-Plane 12")
                .join("Output")
                .join("preferences"),
            PathBuf::from("/Applications/X-Plane 12/Output/preferences"),
        ];

        for path in paths {
            if path.exists() || fs::create_dir_all(&path).is_ok() {
                return Some(path);
            }
        }
    }

    None
}

/// Load configuration from file, or create default if not found
pub fn load_config() -> PluginConfig {
    let config_path = match get_preferences_path() {
        Some(mut path) => {
            path.push("XHoneycombBravo.cfg");
            path
        }
        None => {
            xdebug!("Could not find X-Plane preferences directory, using default config");
            return PluginConfig::default();
        }
    };

    xdebug!("Config path: {:?}", config_path);

    // Try to load existing config
    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<PluginConfig>(&contents) {
                Ok(config) => {
                    xdebug!("Loaded configuration from {:?}", config_path);
                    return config;
                }
                Err(e) => {
                    xdebug!("Error parsing config file: {}. Using defaults.", e);
                }
            },
            Err(e) => {
                xdebug!("Error reading config file: {}. Using defaults.", e);
            }
        }
    }

    // Create default config file
    let default_config = PluginConfig::default();
    if let Ok(toml_string) = toml::to_string_pretty(&default_config) {
        if let Err(e) = fs::write(&config_path, toml_string) {
            xdebug!("Could not write default config: {}", e);
        } else {
            xdebug!("Created default configuration at {:?}", config_path);
        }
    }

    default_config
}

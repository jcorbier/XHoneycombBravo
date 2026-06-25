// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::xdebug;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
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

    #[serde(default)]
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

/// One field per annunciator LED, field names matching the panel silkscreen.
/// `serde(alias = …)` keeps pre-rename configs loading without manual migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnunciatorsConfig {
    pub master_warning: String,
    pub engine_fire: String,
    #[serde(alias = "oil_pressure_low")]
    pub low_oil_pressure: String,
    #[serde(alias = "fuel_pressure_low")]
    pub low_fuel_pressure: String,
    pub anti_ice: String,
    #[serde(alias = "starter")]
    pub starter_engaged: String,
    pub apu: String,
    pub master_caution: String,
    pub vacuum: String,
    #[serde(alias = "hydraulic_pressure")]
    pub low_hyd_pressure: String,
    pub aux_fuel_pump_left: String,
    pub aux_fuel_pump_right: String,
    #[serde(alias = "low_voltage")]
    pub low_volts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// When false, the plugin does not open the Bravo over HID at all.
    #[serde(default = "default_leds_enabled")]
    pub leds_enabled: bool,
    pub bus_voltage: String,
    /// Drives the PARKING BRAKE annunciator. Historical alias `wheel_brake`
    /// still loads, but the value must point at a parking-brake dataref.
    #[serde(alias = "wheel_brake")]
    pub parking_brake: String,
    pub canopy: String,
    pub doors: String,
    pub cabin_door: String,
}

fn default_leds_enabled() -> bool {
    true
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
            trim_wheel: TrimWheelConfig::default(),
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
        low_oil_pressure: "sim/cockpit2/annunciators/oil_pressure_low".to_string(),
        low_fuel_pressure: "sim/cockpit2/annunciators/fuel_pressure_low".to_string(),
        anti_ice: "sim/cockpit2/annunciators/pitot_heat".to_string(),
        starter_engaged: "sim/cockpit2/engine/actuators/starter_hit".to_string(),
        apu: "sim/cockpit2/electrical/APU_running".to_string(),
        master_caution: "sim/cockpit2/annunciators/master_caution".to_string(),
        vacuum: "sim/cockpit2/annunciators/low_vacuum".to_string(),
        low_hyd_pressure: "sim/cockpit2/annunciators/hydraulic_pressure".to_string(),
        aux_fuel_pump_left: "sim/cockpit2/fuel/transfer_pump_left".to_string(),
        aux_fuel_pump_right: "sim/cockpit2/fuel/transfer_pump_right".to_string(),
        low_volts: "sim/cockpit2/annunciators/low_voltage".to_string(),
    }
}

fn default_system() -> SystemConfig {
    SystemConfig {
        leds_enabled: default_leds_enabled(),
        bus_voltage: "sim/cockpit2/electrical/bus_volts".to_string(),
        parking_brake: "sim/cockpit2/controls/parking_brake_ratio".to_string(),
        canopy: "sim/flightmodel2/misc/canopy_open_ratio".to_string(),
        doors: "sim/flightmodel2/misc/door_open_ratio".to_string(),
        cabin_door: "sim/cockpit2/annunciators/cabin_door_open".to_string(),
    }
}

/// X-Plane's preferences directory. `XPLMGetPrefsPath` returns a path to a
/// file *inside* that directory; we strip the file name.
fn get_preferences_path() -> Option<PathBuf> {
    // SDK requires a buffer of at least 512 bytes.
    let mut buf = [0u8; 1024];
    unsafe {
        xplm_sys::XPLMGetPrefsPath(buf.as_mut_ptr().cast::<std::os::raw::c_char>());
    }
    let path = CStr::from_bytes_until_nul(&buf).ok()?.to_str().ok()?;
    let dir = PathBuf::from(path).parent()?.to_path_buf();
    if !dir.exists() {
        fs::create_dir_all(&dir).ok()?;
    }
    Some(dir)
}

/// Load `XHoneycombBravo.cfg`, falling back to defaults (and writing them) on
/// first run.
pub fn load_config() -> PluginConfig {
    let Some(mut config_path) = get_preferences_path() else {
        xdebug!("No X-Plane preferences directory; using default config");
        return PluginConfig::default();
    };
    config_path.push("XHoneycombBravo.cfg");
    xdebug!("Config path: {:?}", config_path);

    if config_path.exists() {
        match fs::read_to_string(&config_path).and_then(|s| {
            toml::from_str::<PluginConfig>(&s)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }) {
            Ok(config) => {
                xdebug!("Loaded configuration from {:?}", config_path);
                return config;
            }
            Err(e) => xdebug!("Could not load {:?}: {}. Using defaults.", config_path, e),
        }
    }

    let default_config = PluginConfig::default();
    match toml::to_string_pretty(&default_config) {
        Ok(toml_string) => match fs::write(&config_path, toml_string) {
            Ok(()) => xdebug!("Created default configuration at {:?}", config_path),
            Err(e) => xdebug!("Could not write default config: {}", e),
        },
        Err(e) => xdebug!("Could not serialize default config: {}", e),
    }
    default_config
}

// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{protocol::DeviceModel, xdebug};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
    /// Select the exact Honeycomb USB device. Existing configurations default
    /// to the original Bravo for backward compatibility.
    #[serde(default)]
    pub device_model: DeviceModel,
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
        device_model: DeviceModel::default(),
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

/// Load `XHoneycombBravo.cfg`, writing defaults only when the file is absent.
/// Invalid existing files are left untouched and stop plugin startup.
pub fn load_config() -> io::Result<PluginConfig> {
    let Some(mut config_path) = get_preferences_path() else {
        xdebug!("No X-Plane preferences directory; using default config");
        return Ok(PluginConfig::default());
    };
    config_path.push("XHoneycombBravo.cfg");
    xdebug!("Config path: {:?}", config_path);

    let existed = config_path.exists();
    let config = load_config_at(&config_path)?;
    if existed {
        xdebug!("Loaded configuration from {:?}", config_path);
    } else {
        xdebug!("Created default configuration at {:?}", config_path);
    }
    Ok(config)
}

fn load_config_at(path: &Path) -> io::Result<PluginConfig> {
    match fs::read_to_string(path) {
        Ok(text) => {
            toml::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let config = PluginConfig::default();
            let text = toml::to_string_pretty(&config).map_err(io::Error::other)?;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?
                .write_all(text.as_bytes())?;
            Ok(config)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "xhoneycomb-config-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn config(&self) -> PathBuf {
            self.0.join("XHoneycombBravo.cfg")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_gets_original_bravo_defaults() {
        let dir = Scratch::new();
        let config = load_config_at(&dir.config()).unwrap();
        assert_eq!(config.system.device_model, DeviceModel::Bravo);
        assert!(dir.config().is_file());
    }

    #[test]
    fn legacy_config_defaults_to_original_without_rewriting() {
        let dir = Scratch::new();
        let text = toml::to_string_pretty(&PluginConfig::default())
            .unwrap()
            .replace("device_model = \"bravo\"\n", "");
        fs::write(dir.config(), &text).unwrap();
        let config = load_config_at(&dir.config()).unwrap();
        assert_eq!(config.system.device_model, DeviceModel::Bravo);
        assert_eq!(fs::read_to_string(dir.config()).unwrap(), text);
    }

    #[test]
    fn lite_config_loads_without_rewriting() {
        let dir = Scratch::new();
        let mut config = PluginConfig::default();
        config.system.device_model = DeviceModel::BravoLite;
        let text = toml::to_string_pretty(&config).unwrap();
        fs::write(dir.config(), &text).unwrap();
        let loaded = load_config_at(&dir.config()).unwrap();
        assert_eq!(loaded.system.device_model, DeviceModel::BravoLite);
        assert_eq!(fs::read_to_string(dir.config()).unwrap(), text);
    }

    #[test]
    fn invalid_model_is_rejected_without_rewriting() {
        let dir = Scratch::new();
        let text = toml::to_string_pretty(&PluginConfig::default())
            .unwrap()
            .replace("device_model = \"bravo\"", "device_model = \"alpha\"");
        fs::write(dir.config(), &text).unwrap();
        assert!(load_config_at(&dir.config()).is_err());
        assert_eq!(fs::read_to_string(dir.config()).unwrap(), text);
    }
}

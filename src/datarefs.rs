// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::config::PluginConfig;
use crate::xdebug;
use xplm::data::borrowed::DataRef;
use xplm::data::ReadOnly;

/// Helper to check if an autopilot state is active (>= 1)
pub fn get_ap_state(value: i32) -> bool {
    value >= 1
}

/// Helper to check if any element in an array is true (== 1)
pub fn array_has_true(array: &[i32]) -> bool {
    array.contains(&1)
}

/// Helper to convert int to bool
pub fn int_to_bool(value: i32) -> bool {
    value != 0
}

/// Helper to convert float to bool
pub fn float_to_bool(value: f32) -> bool {
    value > 0.0
}

/// Dataref accessors loaded dynamically from configuration
pub struct DataRefs {
    // Bus voltage - master LED switch
    pub bus_voltage: Option<DataRef<[f32], ReadOnly>>,

    // Autopilot
    pub hdg: Option<DataRef<i32, ReadOnly>>,
    pub nav: Option<DataRef<i32, ReadOnly>>,
    pub apr: Option<DataRef<i32, ReadOnly>>,
    pub rev: Option<DataRef<i32, ReadOnly>>,
    pub alt: Option<DataRef<i32, ReadOnly>>,
    pub vs: Option<DataRef<i32, ReadOnly>>,
    pub ias: Option<DataRef<i32, ReadOnly>>,
    pub ap: Option<DataRef<i32, ReadOnly>>,

    // Landing gear
    pub gear: Option<DataRef<[f32], ReadOnly>>,

    // Annunciator panel - top row
    pub master_warn: Option<DataRef<i32, ReadOnly>>,
    pub fire: Option<DataRef<[i32], ReadOnly>>,
    pub oil_low_p: Option<DataRef<[i32], ReadOnly>>,
    pub fuel_low_p: Option<DataRef<[i32], ReadOnly>>,
    pub anti_ice: Option<DataRef<i32, ReadOnly>>,
    pub starter: Option<DataRef<[i32], ReadOnly>>,
    pub apu: Option<DataRef<i32, ReadOnly>>,

    // Annunciator panel - bottom row
    pub master_caution: Option<DataRef<i32, ReadOnly>>,
    pub vacuum: Option<DataRef<i32, ReadOnly>>,
    pub hydro_low_p: Option<DataRef<i32, ReadOnly>>,
    pub aux_fuel_pump_l: Option<DataRef<i32, ReadOnly>>,
    pub aux_fuel_pump_r: Option<DataRef<i32, ReadOnly>>,
    pub wheel_brake: Option<DataRef<f32, ReadOnly>>,
    pub volt_low: Option<DataRef<i32, ReadOnly>>,
    pub canopy: Option<DataRef<f32, ReadOnly>>,
    pub doors: Option<DataRef<[f32], ReadOnly>>,
    pub cabin_door: Option<DataRef<i32, ReadOnly>>,
}

impl DataRefs {
    pub fn new(config: &PluginConfig) -> Self {
        xdebug!("Loading datarefs from configuration...");

        DataRefs {
            bus_voltage: DataRef::find(&config.system.bus_voltage).ok(),

            hdg: DataRef::find(&config.autopilot.hdg).ok(),
            nav: DataRef::find(&config.autopilot.nav).ok(),
            apr: DataRef::find(&config.autopilot.apr).ok(),
            rev: DataRef::find(&config.autopilot.rev).ok(),
            alt: DataRef::find(&config.autopilot.alt).ok(),
            vs: DataRef::find(&config.autopilot.vs).ok(),
            ias: DataRef::find(&config.autopilot.ias).ok(),
            ap: DataRef::find(&config.autopilot.ap).ok(),

            gear: DataRef::find(&config.landing_gear.gear).ok(),

            master_warn: DataRef::find(&config.annunciators.master_warning).ok(),
            fire: DataRef::find(&config.annunciators.engine_fire).ok(),
            oil_low_p: DataRef::find(&config.annunciators.oil_pressure_low).ok(),
            fuel_low_p: DataRef::find(&config.annunciators.fuel_pressure_low).ok(),
            anti_ice: DataRef::find(&config.annunciators.anti_ice).ok(),
            starter: DataRef::find(&config.annunciators.starter).ok(),
            apu: DataRef::find(&config.annunciators.apu).ok(),

            master_caution: DataRef::find(&config.annunciators.master_caution).ok(),
            vacuum: DataRef::find(&config.annunciators.vacuum).ok(),
            hydro_low_p: DataRef::find(&config.annunciators.hydraulic_pressure).ok(),
            aux_fuel_pump_l: DataRef::find(&config.annunciators.aux_fuel_pump_left).ok(),
            aux_fuel_pump_r: DataRef::find(&config.annunciators.aux_fuel_pump_right).ok(),
            wheel_brake: DataRef::find(&config.system.wheel_brake).ok(),
            volt_low: DataRef::find(&config.annunciators.low_voltage).ok(),
            canopy: DataRef::find(&config.system.canopy).ok(),
            doors: DataRef::find(&config.system.doors).ok(),
            cabin_door: DataRef::find(&config.system.cabin_door).ok(),
        }
    }
}

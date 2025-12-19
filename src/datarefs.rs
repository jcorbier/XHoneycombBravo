// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: MIT

use xplm::data::borrowed::{DataRef, FindError};
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

/// Dataref accessors for all monitored datarefs
pub struct DataRefs {
    // Bus voltage - master LED switch
    pub bus_voltage: DataRef<[f32], ReadOnly>,

    // Autopilot - most are integers (0=off, 1=armed, 2=active)
    pub hdg: DataRef<i32, ReadOnly>,
    pub nav: DataRef<i32, ReadOnly>,
    pub apr: DataRef<i32, ReadOnly>,
    pub rev: DataRef<i32, ReadOnly>,
    pub alt: DataRef<i32, ReadOnly>,
    pub vs: DataRef<i32, ReadOnly>,
    pub ias: DataRef<i32, ReadOnly>,
    pub ap: DataRef<i32, ReadOnly>,

    // Landing gear - array of floats (0.0=up, 1.0=down)
    pub gear: DataRef<[f32], ReadOnly>,

    // Annunciator panel - top row
    pub master_warn: DataRef<i32, ReadOnly>,
    pub fire: DataRef<[i32], ReadOnly>, // Array - one per engine
    pub oil_low_p: DataRef<[i32], ReadOnly>, // Array - one per engine
    pub fuel_low_p: DataRef<[i32], ReadOnly>, // Array - one per engine
    pub anti_ice: DataRef<i32, ReadOnly>,
    pub starter: DataRef<[i32], ReadOnly>, // Array - one per engine
    pub apu: DataRef<i32, ReadOnly>,

    // Annunciator panel - bottom row
    pub master_caution: DataRef<i32, ReadOnly>,
    pub vacuum: DataRef<i32, ReadOnly>,
    pub hydro_low_p: DataRef<i32, ReadOnly>,
    pub aux_fuel_pump_l: DataRef<i32, ReadOnly>,
    pub aux_fuel_pump_r: DataRef<i32, ReadOnly>,
    pub wheel_brake: DataRef<f32, ReadOnly>,
    pub volt_low: DataRef<i32, ReadOnly>,
    pub canopy: DataRef<f32, ReadOnly>,
    pub doors: DataRef<[f32], ReadOnly>, // Array - multiple doors
    pub cabin_door: DataRef<i32, ReadOnly>,
}

impl DataRefs {
    pub fn new() -> Result<Self, FindError> {
        Ok(DataRefs {
            bus_voltage: DataRef::find("sim/cockpit2/electrical/bus_volts")?,

            hdg: DataRef::find("sim/cockpit2/autopilot/heading_mode")?,
            nav: DataRef::find("sim/cockpit2/autopilot/nav_status")?,
            apr: DataRef::find("sim/cockpit2/autopilot/approach_status")?,
            rev: DataRef::find("sim/cockpit2/autopilot/backcourse_status")?,
            alt: DataRef::find("sim/cockpit2/autopilot/altitude_hold_status")?,
            vs: DataRef::find("sim/cockpit2/autopilot/vvi_status")?,
            ias: DataRef::find("sim/cockpit2/autopilot/autothrottle_on")?,
            ap: DataRef::find("sim/cockpit2/autopilot/servos_on")?,

            gear: DataRef::find("sim/flightmodel2/gear/deploy_ratio")?,

            master_warn: DataRef::find("sim/cockpit2/annunciators/master_warning")?,
            fire: DataRef::find("sim/cockpit2/annunciators/engine_fires")?,
            oil_low_p: DataRef::find("sim/cockpit2/annunciators/oil_pressure_low")?,
            fuel_low_p: DataRef::find("sim/cockpit2/annunciators/fuel_pressure_low")?,
            anti_ice: DataRef::find("sim/cockpit2/annunciators/pitot_heat")?,
            starter: DataRef::find("sim/cockpit2/engine/actuators/starter_hit")?,
            apu: DataRef::find("sim/cockpit2/electrical/APU_running")?,

            master_caution: DataRef::find("sim/cockpit2/annunciators/master_caution")?,
            vacuum: DataRef::find("sim/cockpit2/annunciators/low_vacuum")?,
            hydro_low_p: DataRef::find("sim/cockpit2/annunciators/hydraulic_pressure")?,
            aux_fuel_pump_l: DataRef::find("sim/cockpit2/fuel/transfer_pump_left")?,
            aux_fuel_pump_r: DataRef::find("sim/cockpit2/fuel/transfer_pump_right")?,
            wheel_brake: DataRef::find("sim/cockpit2/controls/wheel_brake_ratio")?,
            volt_low: DataRef::find("sim/cockpit2/annunciators/low_voltage")?,
            canopy: DataRef::find("sim/flightmodel2/misc/canopy_open_ratio")?,
            doors: DataRef::find("sim/flightmodel2/misc/door_open_ratio")?,
            cabin_door: DataRef::find("sim/cockpit2/annunciators/cabin_door_open")?,
        })
    }
}

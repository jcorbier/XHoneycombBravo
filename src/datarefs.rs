// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dataref accessors loaded from [`PluginConfig`], plus bool-coercion helpers
//! the LED layer uses to turn raw dataref values into "lit" decisions.

use crate::config::PluginConfig;
use crate::xdebug;
use xplm::data::ReadOnly;
use xplm::data::borrowed::DataRef;

/// Autopilot mode (0 off / 1 armed / 2 captured): lit on armed-or-better.
pub fn get_ap_state(value: i32) -> bool {
    value >= 1
}

/// True iff any element equals 1. Used for per-engine annunciator arrays.
pub fn array_has_true(array: &[i32]) -> bool {
    array.contains(&1)
}

/// Coerce an X-Plane "boolean" int (0 / 1) to a Rust `bool`.
pub fn int_to_bool(value: i32) -> bool {
    value != 0
}

/// Coerce a continuous-ratio float to a `bool` (any positive value → true).
pub fn float_to_bool(value: f32) -> bool {
    value > 0.0
}

/// Dataref handles loaded from configuration.
///
/// SAFETY: the underlying `XPLMDataRef` pointers are valid for the plugin's
/// lifetime and X-Plane invokes plugin callbacks from a single thread, so the
/// `Send + Sync` impls below are sound and let us hold this in a static.
pub struct DataRefs {
    // Bus voltage drives the master "panel on/off" switch.
    pub bus_voltage: Option<DataRef<[f32], ReadOnly>>,

    // Autopilot row (panel labels: HDG, NAV, APR, REV, ALT, VS, IAS, AUTO PILOT).
    pub hdg: Option<DataRef<i32, ReadOnly>>,
    pub nav: Option<DataRef<i32, ReadOnly>>,
    pub apr: Option<DataRef<i32, ReadOnly>>,
    pub rev: Option<DataRef<i32, ReadOnly>>,
    pub alt: Option<DataRef<i32, ReadOnly>>,
    pub vs: Option<DataRef<i32, ReadOnly>>,
    pub ias: Option<DataRef<i32, ReadOnly>>,
    pub ap: Option<DataRef<i32, ReadOnly>>,

    // Landing-gear strip (3 dual-color LEDs: L, N, R).
    pub gear: Option<DataRef<[f32], ReadOnly>>,

    // Annunciator top row (left → right): field names match panel labels.
    pub master_warning: Option<DataRef<i32, ReadOnly>>,
    pub engine_fire: Option<DataRef<[i32], ReadOnly>>,
    pub low_oil_pressure: Option<DataRef<[i32], ReadOnly>>,
    pub low_fuel_pressure: Option<DataRef<[i32], ReadOnly>>,
    pub anti_ice: Option<DataRef<i32, ReadOnly>>,
    pub starter_engaged: Option<DataRef<[i32], ReadOnly>>,
    pub apu: Option<DataRef<i32, ReadOnly>>,

    // Annunciator bottom row (left → right): field names match panel labels.
    pub master_caution: Option<DataRef<i32, ReadOnly>>,
    pub vacuum: Option<DataRef<i32, ReadOnly>>,
    pub low_hyd_pressure: Option<DataRef<i32, ReadOnly>>,
    pub aux_fuel_pump_left: Option<DataRef<i32, ReadOnly>>,
    pub aux_fuel_pump_right: Option<DataRef<i32, ReadOnly>>,
    pub parking_brake: Option<DataRef<f32, ReadOnly>>,
    pub low_volts: Option<DataRef<i32, ReadOnly>>,

    // Auxiliary inputs feeding the DOOR annunciator (any-of).
    pub canopy: Option<DataRef<f32, ReadOnly>>,
    pub doors: Option<DataRef<[f32], ReadOnly>>,
    pub cabin_door: Option<DataRef<i32, ReadOnly>>,
}

// SAFETY: see DataRefs doc comment.
unsafe impl Send for DataRefs {}
unsafe impl Sync for DataRefs {}

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

            master_warning: DataRef::find(&config.annunciators.master_warning).ok(),
            engine_fire: DataRef::find(&config.annunciators.engine_fire).ok(),
            low_oil_pressure: DataRef::find(&config.annunciators.low_oil_pressure).ok(),
            low_fuel_pressure: DataRef::find(&config.annunciators.low_fuel_pressure).ok(),
            anti_ice: DataRef::find(&config.annunciators.anti_ice).ok(),
            starter_engaged: DataRef::find(&config.annunciators.starter_engaged).ok(),
            apu: DataRef::find(&config.annunciators.apu).ok(),

            master_caution: DataRef::find(&config.annunciators.master_caution).ok(),
            vacuum: DataRef::find(&config.annunciators.vacuum).ok(),
            low_hyd_pressure: DataRef::find(&config.annunciators.low_hyd_pressure).ok(),
            aux_fuel_pump_left: DataRef::find(&config.annunciators.aux_fuel_pump_left).ok(),
            aux_fuel_pump_right: DataRef::find(&config.annunciators.aux_fuel_pump_right).ok(),
            parking_brake: DataRef::find(&config.system.parking_brake).ok(),
            low_volts: DataRef::find(&config.annunciators.low_volts).ok(),
            canopy: DataRef::find(&config.system.canopy).ok(),
            doors: DataRef::find(&config.system.doors).ok(),
            cabin_door: DataRef::find(&config.system.cabin_door).ok(),
        }
    }
}

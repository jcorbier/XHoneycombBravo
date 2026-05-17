// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

//! LED logic: read aircraft datarefs, decide which Bravo LEDs should be lit,
//! and emit a diagnostic line whenever that decision changes.

use crate::datarefs::{DataRefs, array_has_true, float_to_bool, get_ap_state, int_to_bool};
use crate::hid::{
    ALL_LEDS, BANK_COUNT, BravoDevice, LED_ANC_ANTI_ICE, LED_ANC_APU, LED_ANC_AUX_FUEL_PUMP,
    LED_ANC_DOOR, LED_ANC_ENGINE_FIRE, LED_ANC_LOW_FUEL_PRESSURE, LED_ANC_LOW_HYD_PRESSURE,
    LED_ANC_LOW_OIL_PRESSURE, LED_ANC_LOW_VOLTS, LED_ANC_MASTER_CAUTION, LED_ANC_MASTER_WARNING,
    LED_ANC_PARKING_BRAKE, LED_ANC_STARTER_ENGAGED, LED_ANC_VACUUM, LED_AP_ALT, LED_AP_APR,
    LED_AP_AUTOPILOT, LED_AP_HDG, LED_AP_IAS, LED_AP_NAV, LED_AP_REV, LED_AP_VS, LED_LDG_L_GREEN,
    LED_LDG_L_RED, LED_LDG_N_GREEN, LED_LDG_N_RED, LED_LDG_R_GREEN, LED_LDG_R_RED, Led, get_device,
};
use crate::xdebug;
use std::fmt::Write as _;
use std::sync::{LazyLock, Mutex};
use xplm::data::borrowed::DataRef;
use xplm::data::{ArrayRead, DataRead, ReadOnly};

/// Maps the X-Plane gear vector [nose, left, right] → (green, red) on the panel.
const GEAR_LEDS: [(Led, Led); 3] = [
    (LED_LDG_N_GREEN, LED_LDG_N_RED),
    (LED_LDG_L_GREEN, LED_LDG_L_RED),
    (LED_LDG_R_GREEN, LED_LDG_R_RED),
];

/// Update all LED states based on current dataref values.
pub fn handle_led_changes(datarefs: &DataRefs) {
    let Ok(mut device) = get_device().lock() else {
        return;
    };

    let bus_volts = datarefs
        .bus_voltage
        .as_ref()
        .and_then(|d| d.as_vec().first().copied())
        .unwrap_or(0.0);
    let powered = bus_volts > 0.0;

    if powered {
        update_autopilot_leds(&mut device, datarefs);
        update_gear_leds(&mut device, datarefs);
        update_annunciator_leds(&mut device, datarefs);
    } else {
        // No bus voltage → panel dark. `flush` no-ops once the bytes match,
        // so calling this every unpowered tick is effectively free.
        device.all_leds_off();
    }

    log_state_if_changed(&device, datarefs, powered, bus_volts);

    device.flush();
}

fn update_autopilot_leds(device: &mut BravoDevice, d: &DataRefs) {
    set_from_int(device, LED_AP_HDG, d.hdg.as_ref(), get_ap_state);
    set_from_int(device, LED_AP_NAV, d.nav.as_ref(), get_ap_state);
    set_from_int(device, LED_AP_APR, d.apr.as_ref(), get_ap_state);
    set_from_int(device, LED_AP_REV, d.rev.as_ref(), get_ap_state);
    // ALT: light only when *captured* (2), not when merely armed (1).
    set_from_int(device, LED_AP_ALT, d.alt.as_ref(), |v| v > 1);
    set_from_int(device, LED_AP_VS, d.vs.as_ref(), get_ap_state);
    set_from_int(device, LED_AP_IAS, d.ias.as_ref(), get_ap_state);
    set_from_int(device, LED_AP_AUTOPILOT, d.ap.as_ref(), int_to_bool);
}

fn update_gear_leds(device: &mut BravoDevice, d: &DataRefs) {
    let Some(ref gear_dref) = d.gear else { return };
    let gear = gear_dref.as_vec();
    if gear.len() < 3 {
        return;
    }

    for (i, &pos) in gear.iter().take(3).enumerate() {
        let (green, red) = GEAR_LEDS[i];
        // Tolerant of float jitter (some aircraft report 0.999 / 0.001 at the rails).
        let (green_on, red_on) = if pos <= 0.01 {
            (false, false) // stowed
        } else if pos >= 0.99 {
            (true, false) // deployed
        } else {
            (false, true) // in transit
        };
        device.set_led(green, green_on);
        device.set_led(red, red_on);
    }
}

fn update_annunciator_leds(device: &mut BravoDevice, d: &DataRefs) {
    set_from_int(
        device,
        LED_ANC_MASTER_WARNING,
        d.master_warning.as_ref(),
        int_to_bool,
    );
    set_from_int_array(device, LED_ANC_ENGINE_FIRE, d.engine_fire.as_ref());
    set_from_int_array(
        device,
        LED_ANC_LOW_OIL_PRESSURE,
        d.low_oil_pressure.as_ref(),
    );
    set_from_int_array(
        device,
        LED_ANC_LOW_FUEL_PRESSURE,
        d.low_fuel_pressure.as_ref(),
    );
    set_from_int(device, LED_ANC_ANTI_ICE, d.anti_ice.as_ref(), int_to_bool);
    set_from_int_array(device, LED_ANC_STARTER_ENGAGED, d.starter_engaged.as_ref());
    set_from_int(device, LED_ANC_APU, d.apu.as_ref(), int_to_bool);

    set_from_int(
        device,
        LED_ANC_MASTER_CAUTION,
        d.master_caution.as_ref(),
        int_to_bool,
    );
    set_from_int(device, LED_ANC_VACUUM, d.vacuum.as_ref(), int_to_bool);
    set_from_int(
        device,
        LED_ANC_LOW_HYD_PRESSURE,
        d.low_hyd_pressure.as_ref(),
        int_to_bool,
    );
    set_from_int(device, LED_ANC_LOW_VOLTS, d.low_volts.as_ref(), int_to_bool);

    let aux_fuel_active = aux_fuel_running(d.aux_fuel_pump_left.as_ref())
        || aux_fuel_running(d.aux_fuel_pump_right.as_ref());
    device.set_led(LED_ANC_AUX_FUEL_PUMP, aux_fuel_active);

    if let Some(ref dref) = d.parking_brake {
        device.set_led(LED_ANC_PARKING_BRAKE, float_to_bool(dref.get()));
    }

    device.set_led(LED_ANC_DOOR, any_door_open(d));
}

fn aux_fuel_running(dref: Option<&DataRef<i32, ReadOnly>>) -> bool {
    dref.is_some_and(|d| d.get() == 2)
}

fn any_door_open(d: &DataRefs) -> bool {
    if d.canopy.as_ref().is_some_and(|c| c.get() > 0.01) {
        return true;
    }
    if let Some(ref dref) = d.doors {
        if dref.as_vec().iter().take(10).any(|&p| p > 0.01) {
            return true;
        }
    }
    d.cabin_door.as_ref().is_some_and(|c| int_to_bool(c.get()))
}

fn set_from_int(
    device: &mut BravoDevice,
    led: Led,
    dref: Option<&DataRef<i32, ReadOnly>>,
    to_bool: impl FnOnce(i32) -> bool,
) {
    if let Some(d) = dref {
        device.set_led(led, to_bool(d.get()));
    }
}

fn set_from_int_array(device: &mut BravoDevice, led: Led, dref: Option<&DataRef<[i32], ReadOnly>>) {
    if let Some(d) = dref {
        device.set_led(led, array_has_true(&d.as_vec()));
    }
}

// --- Diagnostic logging ----------------------------------------------------
// One line to Log.txt on each change in (powered, bank bytes); quiet during
// steady flight, useful audit trail when a switch flips or a condition trips.

type StateSnapshot = (bool, [u8; BANK_COUNT]);

static LAST_LOGGED_STATE: LazyLock<Mutex<Option<StateSnapshot>>> =
    LazyLock::new(|| Mutex::new(None));

fn log_state_if_changed(device: &BravoDevice, d: &DataRefs, powered: bool, bus_volts: f32) {
    let banks = device.banks();
    let snapshot = (powered, banks);
    {
        let Ok(mut last) = LAST_LOGGED_STATE.lock() else {
            return;
        };
        if last.as_ref() == Some(&snapshot) {
            return;
        }
        *last = Some(snapshot);
    }

    xdebug!(
        "LED change | pwr={} bus={:.2}V | ON=[{}] | banks=[{:02X} {:02X} {:02X} {:02X}] | {}",
        u8::from(powered),
        bus_volts,
        lit_led_names(&banks),
        banks[0],
        banks[1],
        banks[2],
        banks[3],
        dataref_snapshot(d),
    );
}

/// `+`-joined list of lit-LED labels for a bank snapshot. Driven by
/// [`ALL_LEDS`] so the names can't drift from the constants in `crate::hid`.
fn lit_led_names(banks: &[u8; BANK_COUNT]) -> String {
    let mut on = Vec::new();
    for led in ALL_LEDS {
        if led.is_lit(banks) {
            on.push(led.name());
        }
    }
    if on.is_empty() {
        "(none)".into()
    } else {
        on.join("+")
    }
}

fn opt_int(dref: Option<&DataRef<i32, ReadOnly>>) -> String {
    dref.map_or_else(|| "?".into(), |x| x.get().to_string())
}

fn opt_int_arr(dref: Option<&DataRef<[i32], ReadOnly>>) -> String {
    dref.map_or_else(|| "?".into(), |x| format!("{:?}", x.as_vec()))
}

fn opt_f32(dref: Option<&DataRef<f32, ReadOnly>>) -> String {
    dref.map_or_else(|| "?".into(), |x| format!("{:.2}", x.get()))
}

fn opt_f32_arr(dref: Option<&DataRef<[f32], ReadOnly>>) -> String {
    dref.map_or_else(
        || "?".into(),
        |x| {
            let parts: Vec<String> = x.as_vec().iter().map(|v| format!("{v:.2}")).collect();
            format!("[{}]", parts.join(", "))
        },
    )
}

/// One-line dump of every dataref the LED logic reads. Missing → `?`.
fn dataref_snapshot(d: &DataRefs) -> String {
    let mut s = String::with_capacity(256);
    let _ = write!(s, "dr: ap={}", opt_int(d.ap.as_ref()));
    let _ = write!(s, " hdg={}", opt_int(d.hdg.as_ref()));
    let _ = write!(s, " nav={}", opt_int(d.nav.as_ref()));
    let _ = write!(s, " apr={}", opt_int(d.apr.as_ref()));
    let _ = write!(s, " rev={}", opt_int(d.rev.as_ref()));
    let _ = write!(s, " alt={}", opt_int(d.alt.as_ref()));
    let _ = write!(s, " vs={}", opt_int(d.vs.as_ref()));
    let _ = write!(s, " ias={}", opt_int(d.ias.as_ref()));
    let _ = write!(s, " | gear={}", opt_f32_arr(d.gear.as_ref()));
    let _ = write!(
        s,
        " | master_warning={}",
        opt_int(d.master_warning.as_ref())
    );
    let _ = write!(s, " engine_fire={}", opt_int_arr(d.engine_fire.as_ref()));
    let _ = write!(
        s,
        " low_oil_pressure={}",
        opt_int_arr(d.low_oil_pressure.as_ref())
    );
    let _ = write!(
        s,
        " low_fuel_pressure={}",
        opt_int_arr(d.low_fuel_pressure.as_ref())
    );
    let _ = write!(s, " anti_ice={}", opt_int(d.anti_ice.as_ref()));
    let _ = write!(
        s,
        " starter_engaged={}",
        opt_int_arr(d.starter_engaged.as_ref())
    );
    let _ = write!(s, " apu={}", opt_int(d.apu.as_ref()));
    let _ = write!(
        s,
        " | master_caution={}",
        opt_int(d.master_caution.as_ref())
    );
    let _ = write!(s, " vacuum={}", opt_int(d.vacuum.as_ref()));
    let _ = write!(
        s,
        " low_hyd_pressure={}",
        opt_int(d.low_hyd_pressure.as_ref())
    );
    let _ = write!(
        s,
        " aux_fuel_pump_left={} aux_fuel_pump_right={}",
        opt_int(d.aux_fuel_pump_left.as_ref()),
        opt_int(d.aux_fuel_pump_right.as_ref())
    );
    let _ = write!(s, " parking_brake={}", opt_f32(d.parking_brake.as_ref()));
    let _ = write!(s, " low_volts={}", opt_int(d.low_volts.as_ref()));
    let _ = write!(s, " | canopy={}", opt_f32(d.canopy.as_ref()));
    let _ = write!(s, " doors={}", opt_f32_arr(d.doors.as_ref()));
    let _ = write!(s, " cabin_door={}", opt_int(d.cabin_door.as_ref()));
    s
}

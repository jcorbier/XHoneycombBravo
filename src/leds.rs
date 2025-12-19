// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: MIT

use crate::datarefs::{array_has_true, float_to_bool, get_ap_state, int_to_bool, DataRefs};
use crate::hid::*;
use xplm::data::{ArrayRead, DataRead};

/// Update all LED states based on current dataref values
pub fn handle_led_changes(datarefs: &DataRefs) {
    let mut device = get_device().lock().unwrap();

    if !device.is_connected() {
        return;
    }

    // Read bus voltage to determine if LEDs should be on
    let bus_voltage = datarefs.bus_voltage.as_vec();

    if !bus_voltage.is_empty() && bus_voltage[0] > 0.0 {
        device.set_master_state(true);

        // Autopilot LEDs - all scalars, use .get()
        device.set_led(LED_FCU_HDG, get_ap_state(datarefs.hdg.get()));
        device.set_led(LED_FCU_NAV, get_ap_state(datarefs.nav.get()));
        device.set_led(LED_FCU_APR, get_ap_state(datarefs.apr.get()));
        device.set_led(LED_FCU_REV, get_ap_state(datarefs.rev.get()));
        device.set_led(LED_FCU_ALT, datarefs.alt.get() > 1);
        device.set_led(LED_FCU_VS, get_ap_state(datarefs.vs.get()));
        device.set_led(LED_FCU_IAS, get_ap_state(datarefs.ias.get()));
        device.set_led(LED_FCU_AP, int_to_bool(datarefs.ap.get()));

        // Landing gear LEDs - array
        let gear_vals = datarefs.gear.as_vec();
        if gear_vals.len() >= 3 {
            for (i, &gear_pos) in gear_vals.iter().enumerate().take(3) {
                let (green_led, red_led) = match i {
                    0 => (LED_LDG_N_GREEN, LED_LDG_N_RED),
                    1 => (LED_LDG_L_GREEN, LED_LDG_L_RED),
                    2 => (LED_LDG_R_GREEN, LED_LDG_R_RED),
                    _ => continue,
                };

                if gear_pos == 0.0 {
                    // Gear stowed
                    device.set_led(green_led, false);
                    device.set_led(red_led, false);
                } else if gear_pos == 1.0 {
                    // Gear deployed
                    device.set_led(green_led, true);
                    device.set_led(red_led, false);
                } else {
                    // Gear moving
                    device.set_led(green_led, false);
                    device.set_led(red_led, true);
                }
            }
        }

        // Annunciator panel - top row
        device.set_led(LED_ANC_MSTR_WARNG, int_to_bool(datarefs.master_warn.get()));

        let fire_vals = datarefs.fire.as_vec();
        device.set_led(LED_ANC_ENG_FIRE, array_has_true(&fire_vals));

        let oil_vals = datarefs.oil_low_p.as_vec();
        device.set_led(LED_ANC_OIL, array_has_true(&oil_vals));

        let fuel_vals = datarefs.fuel_low_p.as_vec();
        device.set_led(LED_ANC_FUEL, array_has_true(&fuel_vals));

        device.set_led(LED_ANC_ANTI_ICE, !int_to_bool(datarefs.anti_ice.get()));

        let starter_vals = datarefs.starter.as_vec();
        device.set_led(LED_ANC_STARTER, array_has_true(&starter_vals));

        device.set_led(LED_ANC_APU, int_to_bool(datarefs.apu.get()));

        // Annunciator panel - bottom row
        device.set_led(LED_ANC_MSTR_CTN, int_to_bool(datarefs.master_caution.get()));
        device.set_led(LED_ANC_VACUUM, int_to_bool(datarefs.vacuum.get()));
        device.set_led(LED_ANC_HYD, int_to_bool(datarefs.hydro_low_p.get()));

        let aux_fuel_active =
            datarefs.aux_fuel_pump_l.get() == 2 || datarefs.aux_fuel_pump_r.get() == 2;
        device.set_led(LED_ANC_AUX_FUEL, aux_fuel_active);

        device.set_led(LED_ANC_PRK_BRK, float_to_bool(datarefs.wheel_brake.get()));
        device.set_led(LED_ANC_VOLTS, int_to_bool(datarefs.volt_low.get()));

        // Door LED - check canopy, doors array, and cabin door
        let mut door_open = datarefs.canopy.get() > 0.01;

        if !door_open {
            let doors_vals = datarefs.doors.as_vec();
            for &door_pos in doors_vals.iter().take(10) {
                if door_pos > 0.01 {
                    door_open = true;
                    break;
                }
            }
        }

        if !door_open {
            door_open = int_to_bool(datarefs.cabin_door.get());
        }

        device.set_led(LED_ANC_DOOR, door_open);
    } else if device.get_master_state() {
        // No bus voltage, disable all LEDs
        device.set_master_state(false);
        device.all_leds_off();
    }

    // Send HID data if buffer was modified
    device.flush_if_modified();
}

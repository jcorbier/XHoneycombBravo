// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::datarefs::{array_has_true, float_to_bool, get_ap_state, int_to_bool, DataRefs};
use crate::hid::*;
use xplm::data::{ArrayRead, DataRead};

/// Update all LED states based on current dataref values
pub fn handle_led_changes(datarefs: &DataRefs) {
    let mut device = get_device().lock().unwrap();

    device.tick();

    // Read bus voltage to determine if LEDs should be on
    let bus_voltage_on = datarefs
        .bus_voltage
        .as_ref()
        .map(|dref| {
            let vals = dref.as_vec();
            !vals.is_empty() && vals[0] > 0.0
        })
        .unwrap_or(false);

    if bus_voltage_on {
        device.set_master_state(true);

        // Autopilot LEDs - all scalars
        if let Some(ref dref) = datarefs.hdg {
            device.set_led(LED_FCU_HDG, get_ap_state(dref.get()));
        }
        if let Some(ref dref) = datarefs.nav {
            device.set_led(LED_FCU_NAV, get_ap_state(dref.get()));
        }
        if let Some(ref dref) = datarefs.apr {
            device.set_led(LED_FCU_APR, get_ap_state(dref.get()));
        }
        if let Some(ref dref) = datarefs.rev {
            device.set_led(LED_FCU_REV, get_ap_state(dref.get()));
        }
        if let Some(ref dref) = datarefs.alt {
            device.set_led(LED_FCU_ALT, dref.get() > 1);
        }
        if let Some(ref dref) = datarefs.vs {
            device.set_led(LED_FCU_VS, get_ap_state(dref.get()));
        }
        if let Some(ref dref) = datarefs.ias {
            device.set_led(LED_FCU_IAS, get_ap_state(dref.get()));
        }
        if let Some(ref dref) = datarefs.ap {
            device.set_led(LED_FCU_AP, int_to_bool(dref.get()));
        }

        // Landing gear LEDs - array
        if let Some(ref dref) = datarefs.gear {
            let gear_vals = dref.as_vec();
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
        }

        // Annunciator panel - top row
        if let Some(ref dref) = datarefs.master_warn {
            device.set_led(LED_ANC_MSTR_WARNG, int_to_bool(dref.get()));
        }

        if let Some(ref dref) = datarefs.fire {
            let fire_vals = dref.as_vec();
            device.set_led(LED_ANC_ENG_FIRE, array_has_true(&fire_vals));
        }

        if let Some(ref dref) = datarefs.oil_low_p {
            let oil_vals = dref.as_vec();
            device.set_led(LED_ANC_OIL, array_has_true(&oil_vals));
        }

        if let Some(ref dref) = datarefs.fuel_low_p {
            let fuel_vals = dref.as_vec();
            device.set_led(LED_ANC_FUEL, array_has_true(&fuel_vals));
        }

        if let Some(ref dref) = datarefs.anti_ice {
            device.set_led(LED_ANC_ANTI_ICE, !int_to_bool(dref.get()));
        }

        if let Some(ref dref) = datarefs.starter {
            let starter_vals = dref.as_vec();
            device.set_led(LED_ANC_STARTER, array_has_true(&starter_vals));
        }

        if let Some(ref dref) = datarefs.apu {
            device.set_led(LED_ANC_APU, int_to_bool(dref.get()));
        }

        // Annunciator panel - bottom row
        if let Some(ref dref) = datarefs.master_caution {
            device.set_led(LED_ANC_MSTR_CTN, int_to_bool(dref.get()));
        }

        if let Some(ref dref) = datarefs.vacuum {
            device.set_led(LED_ANC_VACUUM, int_to_bool(dref.get()));
        }

        if let Some(ref dref) = datarefs.hydro_low_p {
            device.set_led(LED_ANC_HYD, int_to_bool(dref.get()));
        }

        let aux_fuel_active = datarefs
            .aux_fuel_pump_l
            .as_ref()
            .map(|d| d.get() == 2)
            .unwrap_or(false)
            || datarefs
                .aux_fuel_pump_r
                .as_ref()
                .map(|d| d.get() == 2)
                .unwrap_or(false);
        device.set_led(LED_ANC_AUX_FUEL, aux_fuel_active);

        if let Some(ref dref) = datarefs.wheel_brake {
            device.set_led(LED_ANC_PRK_BRK, float_to_bool(dref.get()));
        }

        if let Some(ref dref) = datarefs.volt_low {
            device.set_led(LED_ANC_VOLTS, int_to_bool(dref.get()));
        }

        // Door LED - check canopy, doors array, and cabin door
        let mut door_open = datarefs
            .canopy
            .as_ref()
            .map(|d| d.get() > 0.01)
            .unwrap_or(false);

        if !door_open {
            if let Some(ref dref) = datarefs.doors {
                let doors_vals = dref.as_vec();
                for &door_pos in doors_vals.iter().take(10) {
                    if door_pos > 0.01 {
                        door_open = true;
                        break;
                    }
                }
            }
        }

        if !door_open {
            door_open = datarefs
                .cabin_door
                .as_ref()
                .map(|d| int_to_bool(d.get()))
                .unwrap_or(false);
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

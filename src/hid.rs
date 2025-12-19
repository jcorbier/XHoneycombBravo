// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: MIT

use hidapi::{HidApi, HidDevice};
use std::sync::Mutex;
use xplm::debugln;

// Honeycomb Bravo USB identifiers
const VENDOR_ID: u16 = 10571;
const PRODUCT_ID: u16 = 6401;

// LED definitions: (bank, bit)
pub const LED_FCU_HDG: (usize, usize) = (1, 1);
pub const LED_FCU_NAV: (usize, usize) = (1, 2);
pub const LED_FCU_APR: (usize, usize) = (1, 3);
pub const LED_FCU_REV: (usize, usize) = (1, 4);
pub const LED_FCU_ALT: (usize, usize) = (1, 5);
pub const LED_FCU_VS: (usize, usize) = (1, 6);
pub const LED_FCU_IAS: (usize, usize) = (1, 7);
pub const LED_FCU_AP: (usize, usize) = (1, 8);

pub const LED_LDG_L_GREEN: (usize, usize) = (2, 1);
pub const LED_LDG_L_RED: (usize, usize) = (2, 2);
pub const LED_LDG_N_GREEN: (usize, usize) = (2, 3);
pub const LED_LDG_N_RED: (usize, usize) = (2, 4);
pub const LED_LDG_R_GREEN: (usize, usize) = (2, 5);
pub const LED_LDG_R_RED: (usize, usize) = (2, 6);

pub const LED_ANC_MSTR_WARNG: (usize, usize) = (2, 7);
pub const LED_ANC_ENG_FIRE: (usize, usize) = (2, 8);
pub const LED_ANC_OIL: (usize, usize) = (3, 1);
pub const LED_ANC_FUEL: (usize, usize) = (3, 2);
pub const LED_ANC_ANTI_ICE: (usize, usize) = (3, 3);
pub const LED_ANC_STARTER: (usize, usize) = (3, 4);
pub const LED_ANC_APU: (usize, usize) = (3, 5);
pub const LED_ANC_MSTR_CTN: (usize, usize) = (3, 6);
pub const LED_ANC_VACUUM: (usize, usize) = (3, 7);
pub const LED_ANC_HYD: (usize, usize) = (3, 8);

pub const LED_ANC_AUX_FUEL: (usize, usize) = (4, 1);
pub const LED_ANC_PRK_BRK: (usize, usize) = (4, 2);
pub const LED_ANC_VOLTS: (usize, usize) = (4, 3);
pub const LED_ANC_DOOR: (usize, usize) = (4, 4);

/// LED buffer state - 4 banks of 8 bits each
type LedBuffer = [[bool; 9]; 5]; // Index 0 unused, 1-4 are banks, 1-8 are bits

pub struct BravoDevice {
    device: Option<HidDevice>,
    buffer: LedBuffer,
    buffer_modified: bool,
    master_state: bool,
}

impl BravoDevice {
    pub fn new() -> Self {
        let api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                debugln!("HoneycombBravo | Failed to initialize HID API: {}", e);
                return BravoDevice {
                    device: None,
                    buffer: [[false; 9]; 5],
                    buffer_modified: false,
                    master_state: false,
                };
            }
        };

        match api.open(VENDOR_ID, PRODUCT_ID) {
            Ok(device) => {
                debugln!(
                    "HoneycombBravo | Successfully connected to Honeycomb Bravo throttle quadrant"
                );
                let mut bravo = BravoDevice {
                    device: Some(device),
                    buffer: [[false; 9]; 5],
                    buffer_modified: false,
                    master_state: false,
                };
                bravo.all_leds_off();
                bravo.send_hid_data();
                bravo
            }
            Err(e) => {
                debugln!("HoneycombBravo | Error: Unable to connect to the Honeycomb Bravo throttle quadrant: {}", e);
                BravoDevice {
                    device: None,
                    buffer: [[false; 9]; 5],
                    buffer_modified: false,
                    master_state: false,
                }
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    pub fn get_led(&self, led: (usize, usize)) -> bool {
        self.buffer[led.0][led.1]
    }

    pub fn set_led(&mut self, led: (usize, usize), state: bool) {
        if state != self.get_led(led) {
            self.buffer[led.0][led.1] = state;
            self.buffer_modified = true;
        }
    }

    pub fn all_leds_off(&mut self) {
        for bank in 1..=4 {
            for bit in 1..=8 {
                self.buffer[bank][bit] = false;
            }
        }
        self.buffer_modified = true;
    }

    pub fn send_hid_data(&mut self) {
        if !self.buffer_modified {
            return;
        }

        let device = match &self.device {
            Some(d) => d,
            None => return,
        };

        // Convert buffer to byte array
        let mut data = [0u8; 65]; // 1 byte report ID + 64 bytes data
        data[0] = 0; // Report ID

        // Convert each bank (1-4) to a byte
        for (idx, bank_num) in (1..=4).enumerate() {
            let mut byte_value = 0u8;
            for bit in 1..=8 {
                if self.buffer[bank_num][bit] {
                    byte_value |= 1 << (bit - 1);
                }
            }
            data[idx + 1] = byte_value;
        }

        match device.send_feature_report(&data) {
            Ok(_) => {
                self.buffer_modified = false;
            }
            Err(e) => {
                debugln!("HoneycombBravo | Error: Feature report write failed: {}", e);
            }
        }
    }

    pub fn set_master_state(&mut self, state: bool) {
        self.master_state = state;
    }

    pub fn get_master_state(&self) -> bool {
        self.master_state
    }

    pub fn flush_if_modified(&mut self) {
        if self.buffer_modified {
            self.send_hid_data();
        }
    }
}

impl Drop for BravoDevice {
    fn drop(&mut self) {
        self.all_leds_off();
        self.send_hid_data();
    }
}

// Global device instance
static BRAVO_DEVICE: once_cell::sync::Lazy<Mutex<BravoDevice>> =
    once_cell::sync::Lazy::new(|| Mutex::new(BravoDevice::new()));

pub fn get_device() -> &'static Mutex<BravoDevice> {
    &BRAVO_DEVICE
}

// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::xdebug;
use hidapi::{HidApi, HidDevice};
use rusb::{Context, Hotplug, HotplugBuilder, Registration, UsbContext};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    last_attempt: Option<Instant>,
    // rusb / hotplug fields
    rusb_context: Option<Context>,
    #[allow(dead_code)] // Kept alive by scope, used for drop side-effects mainly
    hotplug_registration: Option<Registration<Context>>,
    connect_req: Arc<AtomicBool>,
    disconnect_req: Arc<AtomicBool>,
    has_hotplug: bool,
}

struct HotplugCallback {
    connect_req: Arc<AtomicBool>,
    disconnect_req: Arc<AtomicBool>,
}

impl Hotplug<Context> for HotplugCallback {
    fn device_arrived(&mut self, _device: rusb::Device<Context>) {
        xdebug!("Hotplug: Device arrived event");
        self.connect_req.store(true, Ordering::SeqCst);
    }

    fn device_left(&mut self, _device: rusb::Device<Context>) {
        xdebug!("Hotplug: Device left event");
        self.disconnect_req.store(true, Ordering::SeqCst);
    }
}

impl BravoDevice {
    pub fn new() -> Self {
        let connect_req = Arc::new(AtomicBool::new(false));
        let disconnect_req = Arc::new(AtomicBool::new(false));
        let mut has_hotplug = false;
        let mut rusb_context = None;
        let mut hotplug_registration = None;

        // Initialize rusb hotplug if available
        if rusb::has_hotplug() {
            match Context::new() {
                Ok(ctx) => {
                    xdebug!("libusb context initialized");
                    let callback = HotplugCallback {
                        connect_req: connect_req.clone(),
                        disconnect_req: disconnect_req.clone(),
                    };

                    match HotplugBuilder::new()
                        .vendor_id(VENDOR_ID)
                        .product_id(PRODUCT_ID)
                        .register(&ctx, Box::new(callback))
                    {
                        Ok(reg) => {
                            xdebug!("Hotplug callback registered");
                            hotplug_registration = Some(reg);
                            has_hotplug = true;
                            rusb_context = Some(ctx);
                        }
                        Err(e) => {
                            xdebug!("Failed to register hotplug callback: {}", e);
                        }
                    }
                }
                Err(e) => {
                    xdebug!("Failed to initialize libusb context: {}", e);
                }
            }
        } else {
            xdebug!("Hotplug not supported by libusb on this platform");
        }

        let mut bravo = BravoDevice {
            device: None,
            buffer: [[false; 9]; 5],
            buffer_modified: false,
            master_state: false,
            last_attempt: None,
            rusb_context,
            hotplug_registration,
            connect_req,
            disconnect_req,
            has_hotplug,
        };

        // If hotplug is active, we rely on it.
        // But we should do an initial check or just let try_connect handle it?
        // Let's do an initial try_connect regardless.
        bravo.try_connect();
        bravo
    }

    /// Periodic update method
    pub fn tick(&mut self) {
        if self.has_hotplug {
            if let Some(ref ctx) = self.rusb_context {
                // Poll for hotplug events
                if let Err(e) = ctx.handle_events(Some(Duration::from_millis(0))) {
                    xdebug!("Error handling libusb events: {}", e);
                }
            }

            // Check flags
            if self.disconnect_req.swap(false, Ordering::SeqCst) {
                self.disconnect();
            }

            if self.connect_req.swap(false, Ordering::SeqCst) {
                self.try_connect();
            }
        } else {
            // Polling fallback
            if self.device.is_none() {
                self.try_connect();
            }
        }
    }

    pub fn try_connect(&mut self) -> bool {
        if self.device.is_some() {
            return true;
        }

        let now = Instant::now();
        if let Some(last) = self.last_attempt {
            if now.duration_since(last) < Duration::from_secs(3) {
                return false;
            }
        }

        self.last_attempt = Some(now);

        let api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => {
                xdebug!("Failed to initialize HID API: {}", e);
                return false;
            }
        };

        match api.open(VENDOR_ID, PRODUCT_ID) {
            Ok(device) => {
                xdebug!("Successfully connected to Honeycomb Bravo throttle quadrant");
                self.device = Some(device);
                self.all_leds_off();
                self.send_hid_data();
                true
            }
            Err(e) => {
                xdebug!(
                    "Error: Unable to connect to the Honeycomb Bravo throttle quadrant: {}",
                    e
                );
                false
            }
        }
    }

    pub fn disconnect(&mut self) {
        if self.device.is_some() {
            xdebug!("Device disconnected");
            self.device = None;
            self.last_attempt = None;
        }
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
                xdebug!("Error: Feature report write failed: {}", e);
                self.disconnect();
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

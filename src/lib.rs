// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

mod commands;
mod config;
mod datarefs;
mod hid;
mod leds;

use config::load_config;
use datarefs::DataRefs;
use std::fmt;

use xplm::flight_loop::{FlightLoop, LoopState};
use xplm::plugin::{Plugin, PluginInfo};

use xplm::xplane_plugin;

#[macro_export]
macro_rules! xdebug {
    ($($arg:tt)*) => {
        xplm::debugln!("XHoneycombBravo | {}", format!($($arg)*));
    }
}

// We need unsafe static for datarefs because FlightLoop callbacks can't access plugin state
// This is safe because X-Plane is single-threaded for plugin callbacks
static mut DATAREFS: Option<DataRefs> = None;

// Error type that implements std::error::Error
#[derive(Debug)]
struct PluginError(String);

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PluginError {}

impl From<String> for PluginError {
    fn from(s: String) -> Self {
        PluginError(s)
    }
}

impl From<&str> for PluginError {
    fn from(s: &str) -> Self {
        PluginError(s.to_string())
    }
}

struct HoneycombBravoPlugin {
    flight_loop: Option<FlightLoop>,
}

impl Plugin for HoneycombBravoPlugin {
    type Error = PluginError;

    fn start() -> Result<Self, Self::Error> {
        xdebug!("Plugin starting...");

        // Load configuration
        let config = load_config();

        // Initialize datarefs from configuration
        let datarefs = DataRefs::new(&config);

        unsafe {
            DATAREFS = Some(datarefs);
        }

        // Register custom commands
        commands::register_commands(&config);

        xdebug!("Plugin started successfully");

        Ok(HoneycombBravoPlugin { flight_loop: None })
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        xdebug!("Plugin enabled");

        // Create flight loop for periodic updates (LEDs, HID device polling)
        let mut flight_loop = FlightLoop::new(|_state: &mut LoopState| unsafe {
            if let Some(ref datarefs) = DATAREFS {
                leds::handle_led_changes(datarefs);
            }
        });

        flight_loop.schedule_immediate();
        self.flight_loop = Some(flight_loop);

        Ok(())
    }

    fn disable(&mut self) {
        xdebug!("Plugin disabled");
        self.flight_loop = None;

        // Turn off all LEDs
        let mut device = hid::get_device().lock().unwrap();
        device.all_leds_off();
        device.send_hid_data();
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: "HoneycombBravo".into(),
            signature: "com.jcorbier.honeycombbravo".into(),
            description: "Honeycomb Bravo throttle quadrant plugin for X-Plane 12 (LEDs, rotary encoder, trim wheel, thrust reversers)".into(),
        }
    }
}

xplane_plugin!(HoneycombBravoPlugin);

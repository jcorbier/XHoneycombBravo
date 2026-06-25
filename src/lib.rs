// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

mod commands;
mod config;
mod datarefs;
mod hid;
mod leds;

use commands::OwnedCommand;
use config::load_config;
use datarefs::DataRefs;
use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

use xplm::flight_loop::{FlightLoop, LoopState};
use xplm::plugin::{Plugin, PluginInfo};
use xplm::xplane_plugin;

/// LED-update flight-loop period. 20 Hz feels instant and saves dataref reads.
const FLIGHT_LOOP_INTERVAL: Duration = Duration::from_millis(50);

/// `XPLMGetElapsedTime` wrapped in a safe fn so [`xdebug!`] doesn't trip
/// Rust 2024's `unused_unsafe` lint when expanded inside an outer `unsafe` block.
#[doc(hidden)]
#[must_use]
pub fn __sim_elapsed_seconds() -> f32 {
    // SAFETY: callable from any plugin entry point, no preconditions.
    unsafe { xplm_sys::XPLMGetElapsedTime() }
}

/// Plugin-wide diagnostic logger. Prepends `XHoneycombBravo | t={sim_s:.3}s | `
/// to every line so plugin output lines up with X-Plane's own log stamps
/// (`XPLMDebugString` does not add them).
#[macro_export]
macro_rules! xdebug {
    ($($arg:tt)*) => {
        ::xplm::debugln!(
            "XHoneycombBravo | t={:.3}s | {}",
            $crate::__sim_elapsed_seconds(),
            ::std::format_args!($($arg)*)
        )
    }
}

/// Plugin-wide datarefs cache. Written once in `start()`, read by the flight loop.
static DATAREFS: OnceLock<DataRefs> = OnceLock::new();

#[derive(Debug)]
struct PluginError(String);

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PluginError {}

struct HoneycombBravoPlugin {
    flight_loop: Option<FlightLoop>,
    /// Held purely for its `Drop`: unregisters every command handler at `XPluginStop`.
    #[allow(dead_code)]
    commands: Vec<OwnedCommand>,
}

impl Plugin for HoneycombBravoPlugin {
    type Error = PluginError;

    fn start() -> Result<Self, Self::Error> {
        xdebug!(
            "Plugin starting v{} (built for Rust {})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_RUST_VERSION"),
        );

        let config = load_config();

        DATAREFS
            .set(DataRefs::new(&config))
            .map_err(|_| PluginError("DATAREFS already initialized".to_string()))?;

        let commands = commands::register_commands(&config);

        hid::configure(config.system.leds_enabled);
        if config.system.leds_enabled {
            xdebug!("LED HID enabled (IOKit ephemeral open per update)");
        } else {
            xdebug!("LED HID disabled by config; joystick stays with X-Plane");
        }

        xdebug!("Plugin started successfully");
        Ok(HoneycombBravoPlugin {
            flight_loop: None,
            commands,
        })
    }

    fn enable(&mut self) -> Result<(), Self::Error> {
        xdebug!("Plugin enabled");

        let mut flight_loop = FlightLoop::new(|_state: &mut LoopState| {
            if let Some(datarefs) = DATAREFS.get() {
                leds::handle_led_changes(datarefs);
            }
        });

        flight_loop.schedule_after(FLIGHT_LOOP_INTERVAL);
        self.flight_loop = Some(flight_loop);

        Ok(())
    }

    fn disable(&mut self) {
        xdebug!("Plugin disabled");
        self.flight_loop = None;

        if let Ok(mut device) = hid::get_device().lock() {
            device.all_leds_off();
            device.flush();
        }
    }

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: "HoneycombBravo".into(),
            signature: "com.jcorbier.honeycombbravo".into(),
            description: "Honeycomb Bravo throttle quadrant plugin for X-Plane 12 (LEDs, rotary encoder, trim wheel, thrust reversers)".into(),
        }
    }
}

impl Drop for HoneycombBravoPlugin {
    fn drop(&mut self) {
        // Belt-and-suspenders cleanup in case X-Plane skipped `disable`.
        if let Ok(mut device) = hid::get_device().lock() {
            device.all_leds_off();
            device.flush();
        }
        commands::clear_command_state();
        xdebug!("Plugin stopped");
    }
}

xplane_plugin!(HoneycombBravoPlugin);

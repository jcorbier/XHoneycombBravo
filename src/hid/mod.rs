// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

//! LED state model for the Honeycomb Bravo.
//!
//! The Bravo accepts a 64-byte feature report whose first 4 bytes drive the LED banks.
//! We keep a 4-byte mirror of the panel state; whenever it changes we hand the bytes to
//! the platform-specific HID transport (currently [`macos`]).

#[cfg(not(target_os = "macos"))]
compile_error!("XHoneycombBravo currently supports macOS only");

mod macos;

use macos::HidConnection;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// One LED on the Bravo, identified by its bank (0..4) and bit mask.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Led {
    bank: usize,
    mask: u8,
    /// Short label (matches the panel silkscreen) for diagnostic logging.
    name: &'static str,
}

impl Led {
    const fn new(bank: usize, bit: u8, name: &'static str) -> Self {
        debug_assert!(bank < BANK_COUNT);
        debug_assert!(bit < 8);
        Led {
            bank,
            mask: 1 << bit,
            name,
        }
    }

    /// Short label for this LED. Matches the panel silkscreen.
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// True iff this LED is set in the given bank snapshot.
    pub const fn is_lit(&self, banks: &[u8; BANK_COUNT]) -> bool {
        banks[self.bank] & self.mask != 0
    }
}

// ===========================================================================
// Physical panel ⇆ HID bit layout. Names below match the silkscreen verbatim.
//
// Autopilot row (bank 0, left → right):
//     HDG   NAV   APR   REV   ALT   VS   IAS   AUTO PILOT
//
// Landing-gear strip (bank 1, bits 0..5). Each housing has a green and a red
// LED; setting both at once is a hardware no-op (green wins), so the logic
// picks one or none.
//
//        L_GREEN  L_RED   N_GREEN  N_RED   R_GREEN  R_RED
//     +-----------------------------------------------------+
//     |   left gear     |    nose gear   |   right gear     |
//     +-----------------------------------------------------+
//
// Annunciator panel (2 rows × 7 LEDs each). The 14 LEDs spill into banks 1..3
// because the gear strip eats 6 of bank 1's bits; ordering follows the panel's
// reading order: top row left → right, then bottom row left → right.
//
//     +-----------------+-----------------+-----------------+-----------------+-----------------+-----------------+-----------------+
//     | MASTER WARNING  |  ENGINE FIRE    | LOW OIL PRESSURE| LOW FUEL PRESS. |    ANTI ICE     | STARTER ENGAGED |       APU       |
//     +-----------------+-----------------+-----------------+-----------------+-----------------+-----------------+-----------------+
//     | MASTER CAUTION  |     VACUUM      | LOW HYD PRESSURE| AUX FUEL PUMP   |  PARKING BRAKE  |   LOW VOLTS     |      DOOR       |
//     +-----------------+-----------------+-----------------+-----------------+-----------------+-----------------+-----------------+
// ===========================================================================

/// Autopilot row, bank 0. The 8th button is labelled "AUTO PILOT" on the panel
/// (the master AP engage / disengage), hence `LED_AP_AUTOPILOT`.
pub const LED_AP_HDG: Led = Led::new(0, 0, "HDG");
pub const LED_AP_NAV: Led = Led::new(0, 1, "NAV");
pub const LED_AP_APR: Led = Led::new(0, 2, "APR");
pub const LED_AP_REV: Led = Led::new(0, 3, "REV");
pub const LED_AP_ALT: Led = Led::new(0, 4, "ALT");
pub const LED_AP_VS: Led = Led::new(0, 5, "VS");
pub const LED_AP_IAS: Led = Led::new(0, 6, "IAS");
pub const LED_AP_AUTOPILOT: Led = Led::new(0, 7, "AP");

/// Landing-gear strip + first 2 annunciators (top row positions 1-2), bank 1.
pub const LED_LDG_L_GREEN: Led = Led::new(1, 0, "L_GREEN");
pub const LED_LDG_L_RED: Led = Led::new(1, 1, "L_RED");
pub const LED_LDG_N_GREEN: Led = Led::new(1, 2, "N_GREEN");
pub const LED_LDG_N_RED: Led = Led::new(1, 3, "N_RED");
pub const LED_LDG_R_GREEN: Led = Led::new(1, 4, "R_GREEN");
pub const LED_LDG_R_RED: Led = Led::new(1, 5, "R_RED");
pub const LED_ANC_MASTER_WARNING: Led = Led::new(1, 6, "MASTER_WARNING");
pub const LED_ANC_ENGINE_FIRE: Led = Led::new(1, 7, "ENGINE_FIRE");

/// Annunciator top row positions 3-7, then bottom row positions 1-3, bank 2.
pub const LED_ANC_LOW_OIL_PRESSURE: Led = Led::new(2, 0, "LOW_OIL_PRESSURE");
pub const LED_ANC_LOW_FUEL_PRESSURE: Led = Led::new(2, 1, "LOW_FUEL_PRESSURE");
pub const LED_ANC_ANTI_ICE: Led = Led::new(2, 2, "ANTI_ICE");
pub const LED_ANC_STARTER_ENGAGED: Led = Led::new(2, 3, "STARTER_ENGAGED");
pub const LED_ANC_APU: Led = Led::new(2, 4, "APU");
pub const LED_ANC_MASTER_CAUTION: Led = Led::new(2, 5, "MASTER_CAUTION");
pub const LED_ANC_VACUUM: Led = Led::new(2, 6, "VACUUM");
pub const LED_ANC_LOW_HYD_PRESSURE: Led = Led::new(2, 7, "LOW_HYD_PRESSURE");

/// Annunciator bottom row positions 4-7, bank 3 (bits 4-7 unused).
pub const LED_ANC_AUX_FUEL_PUMP: Led = Led::new(3, 0, "AUX_FUEL_PUMP");
pub const LED_ANC_PARKING_BRAKE: Led = Led::new(3, 1, "PARKING_BRAKE");
pub const LED_ANC_LOW_VOLTS: Led = Led::new(3, 2, "LOW_VOLTS");
pub const LED_ANC_DOOR: Led = Led::new(3, 3, "DOOR");

/// Every Bravo LED in physical-panel reading order. Drives the diagnostic
/// logger, so adding a new LED only requires one entry above and one here.
pub const ALL_LEDS: &[Led] = &[
    // Autopilot row
    LED_AP_HDG,
    LED_AP_NAV,
    LED_AP_APR,
    LED_AP_REV,
    LED_AP_ALT,
    LED_AP_VS,
    LED_AP_IAS,
    LED_AP_AUTOPILOT,
    // Landing gear
    LED_LDG_L_GREEN,
    LED_LDG_L_RED,
    LED_LDG_N_GREEN,
    LED_LDG_N_RED,
    LED_LDG_R_GREEN,
    LED_LDG_R_RED,
    // Annunciators, top row then bottom row
    LED_ANC_MASTER_WARNING,
    LED_ANC_ENGINE_FIRE,
    LED_ANC_LOW_OIL_PRESSURE,
    LED_ANC_LOW_FUEL_PRESSURE,
    LED_ANC_ANTI_ICE,
    LED_ANC_STARTER_ENGAGED,
    LED_ANC_APU,
    LED_ANC_MASTER_CAUTION,
    LED_ANC_VACUUM,
    LED_ANC_LOW_HYD_PRESSURE,
    LED_ANC_AUX_FUEL_PUMP,
    LED_ANC_PARKING_BRAKE,
    LED_ANC_LOW_VOLTS,
    LED_ANC_DOOR,
];

pub const BANK_COUNT: usize = 4;
/// Bravo feature-report payload size. IOKit takes the report ID as a separate
/// argument, so the buffer must be payload-only — no leading report-ID byte
/// (raw IOKit ≠ hidapi here). Bytes 0..4 are the LED banks, the rest is padding.
const REPORT_LEN: usize = 64;
/// HID report ID for the LED feature report (Bravo uses an unnumbered report).
const REPORT_ID: u8 = 0;

/// Re-send the current LED state at least this often. Recovers from silent
/// hardware resets (USB unplug/replug, sleep/wake) where the panel goes dark
/// without any logical state change.
const FORCE_REFRESH_INTERVAL: Duration = Duration::from_secs(3);

/// LED panel state mirror.
pub struct BravoDevice {
    connection: HidConnection,
    /// Desired state, kept in sync with X-Plane datarefs.
    banks: [u8; BANK_COUNT],
    /// Last bytes successfully written to hardware. `None` forces a re-send
    /// (first run, last send failed, or LEDs were just re-enabled).
    last_sent: Option<[u8; BANK_COUNT]>,
    /// Deadline for the next mandatory re-send regardless of `last_sent`.
    next_force_refresh: Instant,
    leds_enabled: bool,
}

impl BravoDevice {
    fn new(leds_enabled: bool) -> Self {
        BravoDevice {
            connection: HidConnection::new(),
            banks: [0; BANK_COUNT],
            last_sent: None,
            next_force_refresh: Instant::now() + FORCE_REFRESH_INTERVAL,
            leds_enabled,
        }
    }

    pub fn set_leds_enabled(&mut self, enabled: bool) {
        if self.leds_enabled == enabled {
            return;
        }
        if enabled {
            self.leds_enabled = true;
            // Force a re-sync so the panel reflects current state immediately.
            self.last_sent = None;
        } else {
            // Send the all-off before flipping the flag; `flush` no-ops once disabled.
            self.all_leds_off();
            self.flush();
            self.leds_enabled = false;
        }
    }

    pub fn set_led(&mut self, led: Led, on: bool) {
        let prev = self.banks[led.bank];
        self.banks[led.bank] = if on {
            prev | led.mask
        } else {
            prev & !led.mask
        };
    }

    pub fn all_leds_off(&mut self) {
        self.banks = [0; BANK_COUNT];
    }

    /// Current in-memory bank bytes (used by the diagnostic logger).
    pub fn banks(&self) -> [u8; BANK_COUNT] {
        self.banks
    }

    /// Push panel state to the hardware on state change or every
    /// [`FORCE_REFRESH_INTERVAL`], whichever comes first.
    pub fn flush(&mut self) {
        if !self.leds_enabled {
            return;
        }

        let now = Instant::now();
        let state_changed = self.last_sent.as_ref() != Some(&self.banks);
        let due_for_refresh = now >= self.next_force_refresh;
        if !state_changed && !due_for_refresh {
            return;
        }

        // Banks live at the start of the payload; the rest stays zero. IOKit
        // handles the report-ID byte separately, do NOT prepend it here.
        let mut report = [0u8; REPORT_LEN];
        report[..BANK_COUNT].copy_from_slice(&self.banks);

        if self.connection.send_feature_report(REPORT_ID, &report) {
            self.last_sent = Some(self.banks);
            self.next_force_refresh = now + FORCE_REFRESH_INTERVAL;
        } else {
            // Force a retry on the next flush (after any backoff expires).
            self.last_sent = None;
        }
    }
}

// No `Drop` impl: `BravoDevice` lives in a `OnceLock<Mutex<...>>` static and
// `dlclose` does not drop statics. Cleanup happens explicitly in `Plugin::disable`.

const LEDS_ENABLED_DEFAULT: bool = true;

static BRAVO_DEVICE: OnceLock<Mutex<BravoDevice>> = OnceLock::new();

/// Initialise the global device once with the configured LED enable flag.
/// Subsequent calls update the flag in place.
pub fn configure(leds_enabled: bool) {
    let cell = BRAVO_DEVICE.get_or_init(|| Mutex::new(BravoDevice::new(leds_enabled)));
    if let Ok(mut device) = cell.lock() {
        device.set_leds_enabled(leds_enabled);
    }
}

/// Global device, lazily initialised with [`LEDS_ENABLED_DEFAULT`] if
/// [`configure`] was never called.
pub fn get_device() -> &'static Mutex<BravoDevice> {
    BRAVO_DEVICE.get_or_init(|| Mutex::new(BravoDevice::new(LEDS_ENABLED_DEFAULT)))
}

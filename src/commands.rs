// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

use once_cell::sync::Lazy;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::sync::Mutex;
use xplm::debugln;
use xplm_sys::{
    xplm_CommandBegin, xplm_CommandContinue, XPLMCommandPhase, XPLMCommandRef, XPLMCreateCommand,
    XPLMDataRef, XPLMFindCommand, XPLMFindDataRef, XPLMGetDataf, XPLMGetDatavf,
    XPLMRegisterCommandHandler, XPLMSetDataf, XPLMSetDatavf,
};

/// Autopilot mode for rotary encoder
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutopilotMode {
    Ias,
    Crs,
    Hdg,
    VS,
    Alt,
    COM1Coarse,
    COM1Fine,
}

/// Global mode state
static CURRENT_MODE: Lazy<Mutex<AutopilotMode>> = Lazy::new(|| Mutex::new(AutopilotMode::Ias));

/// Writable datarefs for command system (using raw XPLMDataRef pointers)
pub struct CommandDataRefs {
    pub airspeed_is_mach: XPLMDataRef,
    pub airspeed: XPLMDataRef,
    pub course: XPLMDataRef,
    pub heading: XPLMDataRef,
    pub vs_dial: XPLMDataRef,
    pub altitude: XPLMDataRef,
    pub reversers: XPLMDataRef,
}

// SAFETY: XPLMDataRef pointers are safe to send between threads once obtained from X-Plane.
// The X-Plane SDK guarantees that dataref pointers remain valid for the lifetime of the plugin
// and can be accessed from any thread.
unsafe impl Send for CommandDataRefs {}
unsafe impl Sync for CommandDataRefs {}

static COMMAND_DATAREFS: Lazy<Mutex<Option<CommandDataRefs>>> = Lazy::new(|| Mutex::new(None));

impl CommandDataRefs {
    pub fn new() -> Self {
        unsafe {
            CommandDataRefs {
                airspeed_is_mach: XPLMFindDataRef(
                    CString::new("sim/cockpit2/autopilot/airspeed_is_mach")
                        .unwrap()
                        .as_ptr(),
                ),
                airspeed: XPLMFindDataRef(
                    CString::new("sim/cockpit2/autopilot/airspeed_dial_kts_mach")
                        .unwrap()
                        .as_ptr(),
                ),
                course: XPLMFindDataRef(
                    CString::new("sim/cockpit2/radios/actuators/nav1_obs_deg_mag_pilot")
                        .unwrap()
                        .as_ptr(),
                ),
                heading: XPLMFindDataRef(
                    CString::new("sim/cockpit2/autopilot/heading_dial_deg_mag_pilot")
                        .unwrap()
                        .as_ptr(),
                ),
                vs_dial: XPLMFindDataRef(
                    CString::new("sim/cockpit2/autopilot/vvi_dial_fpm")
                        .unwrap()
                        .as_ptr(),
                ),
                altitude: XPLMFindDataRef(
                    CString::new("sim/cockpit2/autopilot/altitude_dial_ft")
                        .unwrap()
                        .as_ptr(),
                ),
                reversers: XPLMFindDataRef(
                    CString::new("sim/cockpit2/engine/actuators/prop_mode")
                        .unwrap()
                        .as_ptr(),
                ),
            }
        }
    }
}

pub fn get_current_mode() -> AutopilotMode {
    *CURRENT_MODE.lock().unwrap()
}

pub fn set_current_mode(mode: AutopilotMode) {
    *CURRENT_MODE.lock().unwrap() = mode;
    debugln!("HoneycombBravo | Autopilot mode set to {:?}", mode);
}

/// Change autopilot value based on current mode
pub fn change_value(increase: bool) {
    let mode = get_current_mode();
    let sign = if increase { 1.0 } else { -1.0 };

    let datarefs_guard = COMMAND_DATAREFS.lock().unwrap();
    let datarefs = match datarefs_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    unsafe {
        match mode {
            AutopilotMode::Ias => {
                if datarefs.airspeed_is_mach.is_null() || datarefs.airspeed.is_null() {
                    return;
                }

                let mut is_mach = [0.0f32; 1];
                XPLMGetDatavf(datarefs.airspeed_is_mach, is_mach.as_mut_ptr(), 0, 1);
                let current = XPLMGetDataf(datarefs.airspeed);

                if is_mach[0] > 0.5 {
                    // Mach mode - increment by 0.01
                    let new_val = ((current * 100.0).round() + sign) / 100.0;
                    XPLMSetDataf(datarefs.airspeed, new_val.max(0.0));
                } else {
                    // Knots mode - increment by 10
                    let new_val = ((current / 10.0).floor() + sign) * 10.0;
                    XPLMSetDataf(datarefs.airspeed, new_val.max(0.0));
                }
            }
            AutopilotMode::Crs => {
                if datarefs.course.is_null() {
                    return;
                }
                let current = XPLMGetDataf(datarefs.course);
                let mut new_val = current + sign;
                if new_val < 0.0 {
                    new_val = 359.0;
                } else if new_val >= 360.0 {
                    new_val = 0.0;
                }
                XPLMSetDataf(datarefs.course, new_val);
            }
            AutopilotMode::Hdg => {
                if datarefs.heading.is_null() {
                    return;
                }
                let current = XPLMGetDataf(datarefs.heading);
                let mut new_val = current + sign;
                if new_val < 0.0 {
                    new_val = 359.0;
                } else if new_val >= 360.0 {
                    new_val = 0.0;
                }
                XPLMSetDataf(datarefs.heading, new_val);
            }
            AutopilotMode::VS => {
                if datarefs.vs_dial.is_null() {
                    return;
                }
                let current = XPLMGetDataf(datarefs.vs_dial);
                let new_val = ((current / 100.0).floor() + sign) * 100.0;
                XPLMSetDataf(datarefs.vs_dial, new_val);
            }
            AutopilotMode::Alt => {
                if datarefs.altitude.is_null() {
                    return;
                }
                let current = XPLMGetDataf(datarefs.altitude);
                let new_val = ((current / 100.0).floor() + sign) * 100.0;
                XPLMSetDataf(datarefs.altitude, new_val);
            }
            AutopilotMode::COM1Coarse => {
                let cmd_name = if increase {
                    CString::new("sim/radios/stby_com1_coarse_up").unwrap()
                } else {
                    CString::new("sim/radios/stby_com1_coarse_down").unwrap()
                };
                let cmd = XPLMFindCommand(cmd_name.as_ptr());
                if !cmd.is_null() {
                    xplm_sys::XPLMCommandOnce(cmd);
                }
            }
            AutopilotMode::COM1Fine => {
                let cmd_name = if increase {
                    CString::new("sim/radios/stby_com1_fine_up").unwrap()
                } else {
                    CString::new("sim/radios/stby_com1_fine_down").unwrap()
                };
                let cmd = XPLMFindCommand(cmd_name.as_ptr());
                if !cmd.is_null() {
                    xplm_sys::XPLMCommandOnce(cmd);
                }
            }
        }
    }
}

/// Set thrust reverser state for specific engine or all engines
pub fn set_reverser_state(engine: Option<usize>, state: bool) {
    let prop_mode = if state { 3.0 } else { 1.0 };

    let datarefs_guard = COMMAND_DATAREFS.lock().unwrap();
    let datarefs = match datarefs_guard.as_ref() {
        Some(d) => d,
        None => return,
    };

    unsafe {
        if datarefs.reversers.is_null() {
            return;
        }

        let mut reversers_vals = [0.0f32; 8];
        let count = XPLMGetDatavf(datarefs.reversers, reversers_vals.as_mut_ptr(), 0, 8);

        match engine {
            Some(eng) if eng < count as usize => {
                reversers_vals[eng] = prop_mode;
            }
            None => {
                for val in reversers_vals.iter_mut().take(count.min(8) as usize) {
                    *val = prop_mode;
                }
            }
            _ => return,
        }

        XPLMSetDatavf(datarefs.reversers, reversers_vals.as_mut_ptr(), 0, count);
    }
}

/// Command callback for mode selection
unsafe extern "C" fn mode_command_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    if phase == xplm_CommandBegin as XPLMCommandPhase {
        let mode = refcon as usize;
        let autopilot_mode = match mode {
            0 => AutopilotMode::Ias,
            1 => AutopilotMode::Crs,
            2 => AutopilotMode::Hdg,
            3 => AutopilotMode::VS,
            4 => AutopilotMode::Alt,
            5 => AutopilotMode::COM1Coarse,
            6 => AutopilotMode::COM1Fine,
            _ => return 0,
        };
        set_current_mode(autopilot_mode);
    }
    0
}

/// Command callback for value changes
unsafe extern "C" fn value_change_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    if phase == xplm_CommandBegin as XPLMCommandPhase {
        let increase = refcon as usize == 1;
        change_value(increase);
    }
    0
}

/// Command callback for thrust reversers
unsafe extern "C" fn reverser_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    let engine = refcon as isize;
    let state = phase == xplm_CommandBegin as XPLMCommandPhase
        || phase == xplm_CommandContinue as XPLMCommandPhase;

    if engine == -1 {
        set_reverser_state(None, state);
    } else if (0..8).contains(&engine) {
        set_reverser_state(Some(engine as usize), state);
    }

    0
}

/// Register all custom commands
pub fn register_commands() {
    // Initialize command datarefs
    *COMMAND_DATAREFS.lock().unwrap() = Some(CommandDataRefs::new());

    unsafe {
        // Mode selection commands
        let modes = vec![
            (0, "mode_ias", "Set autopilot rotary encoder mode to IAS."),
            (1, "mode_crs", "Set autopilot rotary encoder mode to CRS."),
            (2, "mode_hdg", "Set autopilot rotary encoder mode to HDG."),
            (3, "mode_vs", "Set autopilot rotary encoder mode to VS."),
            (4, "mode_alt", "Set autopilot rotary encoder mode to ALT."),
            (
                5,
                "mode_com1_coarse",
                "Set autopilot rotary encoder mode to COM1_COARSE.",
            ),
            (
                6,
                "mode_com1_fine",
                "Set autopilot rotary encoder mode to COM1_FINE.",
            ),
        ];

        for (mode_id, name, description) in modes {
            let cmd_name = CString::new(format!("HoneycombBravo/{}", name)).unwrap();
            let cmd_desc = CString::new(description).unwrap();

            let cmd = XPLMCreateCommand(cmd_name.as_ptr(), cmd_desc.as_ptr());
            XPLMRegisterCommandHandler(cmd, Some(mode_command_handler), 1, mode_id as *mut c_void);
        }

        // Value change commands
        let increase_cmd = CString::new("HoneycombBravo/increase").unwrap();
        let increase_desc = CString::new(
            "Increase the value of the autopilot mode selected with the rotary encoder.",
        )
        .unwrap();
        let cmd = XPLMCreateCommand(increase_cmd.as_ptr(), increase_desc.as_ptr());
        XPLMRegisterCommandHandler(
            cmd,
            Some(value_change_handler),
            1,
            std::ptr::dangling_mut::<c_void>(),
        );

        let decrease_cmd = CString::new("HoneycombBravo/decrease").unwrap();
        let decrease_desc = CString::new(
            "Decrease the value of the autopilot mode selected with the rotary encoder.",
        )
        .unwrap();
        let cmd = XPLMCreateCommand(decrease_cmd.as_ptr(), decrease_desc.as_ptr());
        XPLMRegisterCommandHandler(
            cmd,
            Some(value_change_handler),
            1,
            std::ptr::null_mut::<c_void>(),
        );

        // Thrust reverser commands
        let all_reversers_cmd = CString::new("HoneycombBravo/thrust_reversers").unwrap();
        let all_reversers_desc = CString::new("Hold all thrust reversers on.").unwrap();
        let cmd = XPLMCreateCommand(all_reversers_cmd.as_ptr(), all_reversers_desc.as_ptr());
        XPLMRegisterCommandHandler(cmd, Some(reverser_handler), 1, -1isize as *mut c_void);

        for i in 0..8 {
            let reverser_cmd =
                CString::new(format!("HoneycombBravo/thrust_reverser_{}", i + 1)).unwrap();
            let reverser_desc =
                CString::new(format!("Hold thrust reverser #{} on.", i + 1)).unwrap();
            let cmd = XPLMCreateCommand(reverser_cmd.as_ptr(), reverser_desc.as_ptr());
            XPLMRegisterCommandHandler(cmd, Some(reverser_handler), 1, i as *mut c_void);
        }
    }

    debugln!("HoneycombBravo | Registered all custom commands");
}

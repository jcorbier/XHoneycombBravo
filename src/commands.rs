// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

//! X-Plane custom commands: rotary encoder mode + value, thrust reversers, trim wheel.

use crate::config::PluginConfig;
use crate::xdebug;
use std::ffi::{CStr, CString};
use std::os::raw::{c_int, c_void};
use std::sync::{LazyLock, Mutex};
use xplm_sys::{
    XPLMCommandCallback_f, XPLMCommandPhase, XPLMCommandRef, XPLMCreateCommand, XPLMDataRef,
    XPLMFindCommand, XPLMFindDataRef, XPLMGetDataf, XPLMGetDatavf, XPLMRegisterCommandHandler,
    XPLMSetDataf, XPLMSetDatavf, XPLMUnregisterCommandHandler, xplm_CommandBegin,
    xplm_CommandContinue,
};

/// Boolean refcon sentinels. The pointer *value* is the payload, never deref'd.
const REFCON_FALSE: *mut c_void = std::ptr::null_mut();
const REFCON_TRUE: *mut c_void = std::ptr::dangling_mut::<c_void>();

/// Maximum engine count we can address (SDK can report up to 8).
const MAX_ENGINES: usize = 8;

/// Which value the rotary encoder is currently controlling. Matches the IAS /
/// CRS / HDG / VS / ALT mode buttons on the Bravo, plus two COM1 tuning modes
/// for use without a physical radio panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AutopilotMode {
    Ias,
    Crs,
    Hdg,
    Vs,
    Alt,
    Com1Coarse,
    Com1Fine,
}

/// Single source of truth for `mode_*` commands: rotary mode, command-name
/// suffix, UI description. `register_commands` registers each entry and uses
/// its slice index as the refcon, so dispatch and registration can't drift.
#[rustfmt::skip]
const MODES: &[(AutopilotMode, &str, &str)] = &[
    (AutopilotMode::Ias,        "mode_ias",         "Set autopilot rotary encoder mode to IAS."),
    (AutopilotMode::Crs,        "mode_crs",         "Set autopilot rotary encoder mode to CRS."),
    (AutopilotMode::Hdg,        "mode_hdg",         "Set autopilot rotary encoder mode to HDG."),
    (AutopilotMode::Vs,         "mode_vs",          "Set autopilot rotary encoder mode to VS."),
    (AutopilotMode::Alt,        "mode_alt",         "Set autopilot rotary encoder mode to ALT."),
    (AutopilotMode::Com1Coarse, "mode_com1_coarse", "Set autopilot rotary encoder mode to COM1 coarse."),
    (AutopilotMode::Com1Fine,   "mode_com1_fine",   "Set autopilot rotary encoder mode to COM1 fine."),
];

static CURRENT_MODE: LazyLock<Mutex<AutopilotMode>> =
    LazyLock::new(|| Mutex::new(AutopilotMode::Ias));

/// Writable datarefs for command system (using raw XPLMDataRef pointers).
pub struct CommandDataRefs {
    pub airspeed_is_mach: XPLMDataRef,
    pub airspeed: XPLMDataRef,
    pub course: XPLMDataRef,
    pub heading: XPLMDataRef,
    pub vs_dial: XPLMDataRef,
    pub altitude: XPLMDataRef,
    pub reversers: XPLMDataRef,
}

// SAFETY: XPLMDataRef pointers are valid for the plugin's lifetime and can be
// read/written from the X-Plane main thread (the only thread that touches them).
unsafe impl Send for CommandDataRefs {}
unsafe impl Sync for CommandDataRefs {}

static COMMAND_DATAREFS: LazyLock<Mutex<Option<CommandDataRefs>>> =
    LazyLock::new(|| Mutex::new(None));

/// Trim wheel state holding the writable dataref and pre-computed delta.
pub struct TrimState {
    pub trim_dataref: XPLMDataRef,
    pub trim_delta: f32,
    pub min_trim: f32,
    pub max_trim: f32,
}

// SAFETY: see CommandDataRefs above.
unsafe impl Send for TrimState {}
unsafe impl Sync for TrimState {}

static TRIM_STATE: LazyLock<Mutex<Option<TrimState>>> = LazyLock::new(|| Mutex::new(None));

impl CommandDataRefs {
    pub fn new() -> Self {
        CommandDataRefs {
            airspeed_is_mach: find_dataref(c"sim/cockpit2/autopilot/airspeed_is_mach"),
            airspeed: find_dataref(c"sim/cockpit2/autopilot/airspeed_dial_kts_mach"),
            course: find_dataref(c"sim/cockpit2/radios/actuators/nav1_obs_deg_mag_pilot"),
            heading: find_dataref(c"sim/cockpit2/autopilot/heading_dial_deg_mag_pilot"),
            vs_dial: find_dataref(c"sim/cockpit2/autopilot/vvi_dial_fpm"),
            altitude: find_dataref(c"sim/cockpit2/autopilot/altitude_dial_ft"),
            reversers: find_dataref(c"sim/cockpit2/engine/actuators/prop_mode"),
        }
    }
}

fn find_dataref(name: &CStr) -> XPLMDataRef {
    unsafe { XPLMFindDataRef(name.as_ptr()) }
}

pub fn get_current_mode() -> AutopilotMode {
    *CURRENT_MODE.lock().unwrap()
}

pub fn set_current_mode(mode: AutopilotMode) {
    *CURRENT_MODE.lock().unwrap() = mode;
    xdebug!("Rotary mode -> {:?}", mode);
}

/// Change autopilot value based on current mode.
pub fn change_value(increase: bool) {
    let mode = get_current_mode();
    let sign = if increase { 1.0 } else { -1.0 };
    let dir = if increase { "+" } else { "-" };
    xdebug!("Rotary turn: dir={dir} mode={mode:?}");

    let datarefs_guard = COMMAND_DATAREFS.lock().unwrap();
    let Some(datarefs) = datarefs_guard.as_ref() else {
        return;
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
                let new_val = if is_mach[0] > 0.5 {
                    ((current * 100.0).round() + sign) / 100.0
                } else {
                    ((current / 10.0).floor() + sign) * 10.0
                };
                XPLMSetDataf(datarefs.airspeed, new_val.max(0.0));
            }
            AutopilotMode::Crs => set_wrapped_angle(datarefs.course, sign),
            AutopilotMode::Hdg => set_wrapped_angle(datarefs.heading, sign),
            AutopilotMode::Vs => set_stepped(datarefs.vs_dial, sign, 100.0),
            AutopilotMode::Alt => set_stepped(datarefs.altitude, sign, 100.0),
            AutopilotMode::Com1Coarse => fire_sdk_command(if increase {
                c"sim/radios/stby_com1_coarse_up"
            } else {
                c"sim/radios/stby_com1_coarse_down"
            }),
            AutopilotMode::Com1Fine => fire_sdk_command(if increase {
                c"sim/radios/stby_com1_fine_up"
            } else {
                c"sim/radios/stby_com1_fine_down"
            }),
        }
    }
}

unsafe fn set_wrapped_angle(dref: XPLMDataRef, sign: f32) {
    if dref.is_null() {
        return;
    }
    unsafe {
        let current = XPLMGetDataf(dref);
        let mut new_val = current + sign;
        if new_val < 0.0 {
            new_val = 359.0;
        } else if new_val >= 360.0 {
            new_val = 0.0;
        }
        XPLMSetDataf(dref, new_val);
    }
}

unsafe fn set_stepped(dref: XPLMDataRef, sign: f32, step: f32) {
    if dref.is_null() {
        return;
    }
    unsafe {
        let current = XPLMGetDataf(dref);
        let new_val = ((current / step).floor() + sign) * step;
        XPLMSetDataf(dref, new_val);
    }
}

unsafe fn fire_sdk_command(name: &CStr) {
    unsafe {
        let cmd = XPLMFindCommand(name.as_ptr());
        if !cmd.is_null() {
            xplm_sys::XPLMCommandOnce(cmd);
        }
    }
}

/// Set thrust reverser state for a specific engine or all engines.
pub fn set_reverser_state(engine: Option<usize>, state: bool) {
    let prop_mode = if state { 3.0 } else { 1.0 };

    let datarefs_guard = COMMAND_DATAREFS.lock().unwrap();
    let Some(datarefs) = datarefs_guard.as_ref() else {
        return;
    };

    unsafe {
        if datarefs.reversers.is_null() {
            return;
        }

        let mut buf = [0.0f32; MAX_ENGINES];
        // Clamp both the request and the returned count to MAX_ENGINES.
        let read = XPLMGetDatavf(datarefs.reversers, buf.as_mut_ptr(), 0, MAX_ENGINES as i32)
            .max(0)
            .min(MAX_ENGINES as i32);
        let read_usize = read as usize;

        match engine {
            Some(eng) if eng < read_usize => buf[eng] = prop_mode,
            Some(_) => return,
            None => {
                for v in &mut buf[..read_usize] {
                    *v = prop_mode;
                }
            }
        }

        XPLMSetDatavf(datarefs.reversers, buf.as_mut_ptr(), 0, read);
    }
}

/// Apply trim wheel delta in the given direction (+1.0 = nose up, -1.0 = nose down).
fn apply_trim(direction: f32) {
    let state_guard = TRIM_STATE.lock().unwrap();
    let Some(state) = state_guard.as_ref() else {
        return;
    };

    unsafe {
        if state.trim_dataref.is_null() {
            return;
        }
        let current = XPLMGetDataf(state.trim_dataref);
        let new_val =
            (current + state.trim_delta * direction).clamp(state.min_trim, state.max_trim);
        XPLMSetDataf(state.trim_dataref, new_val);
    }
}

// --- Command callbacks (extern "C" handlers) ---

unsafe extern "C" fn mode_command_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    if phase == xplm_CommandBegin as XPLMCommandPhase {
        let idx = refcon as usize;
        if let Some(&(mode, _, _)) = MODES.get(idx) {
            set_current_mode(mode);
        }
    }
    0
}

unsafe extern "C" fn value_change_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    if phase == xplm_CommandBegin as XPLMCommandPhase {
        change_value(refcon == REFCON_TRUE);
    }
    0
}

unsafe extern "C" fn reverser_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    let engine = refcon as isize;
    let active = phase == xplm_CommandBegin as XPLMCommandPhase
        || phase == xplm_CommandContinue as XPLMCommandPhase;

    if engine == -1 {
        set_reverser_state(None, active);
    } else if (0..MAX_ENGINES as isize).contains(&engine) {
        set_reverser_state(Some(engine as usize), active);
    }
    0
}

unsafe extern "C" fn trim_command_handler(
    _cmd_ref: XPLMCommandRef,
    phase: XPLMCommandPhase,
    refcon: *mut c_void,
) -> c_int {
    if phase == xplm_CommandBegin as XPLMCommandPhase {
        apply_trim(if refcon == REFCON_TRUE { 1.0 } else { -1.0 });
    }
    0
}

/// A registered command handler that unregisters itself on drop. Without this,
/// the dylib could unload while X-Plane still held our callback pointers, and
/// the next invocation would jump into freed memory.
pub struct OwnedCommand {
    cmd: XPLMCommandRef,
    handler: XPLMCommandCallback_f,
    refcon: *mut c_void,
}

// SAFETY: opaque tokens for X-Plane; never dereferenced; callbacks are single-threaded.
unsafe impl Send for OwnedCommand {}
unsafe impl Sync for OwnedCommand {}

impl OwnedCommand {
    /// Register an X-Plane command and bind a handler with the given refcon.
    ///
    /// The name/description strings are intentionally leaked: the SDK stores
    /// the raw pointers for the command's lifetime, so a stack `CString` would
    /// be a use-after-free.
    fn new(
        name: &str,
        description: &str,
        handler: XPLMCommandCallback_f,
        refcon: *mut c_void,
    ) -> Self {
        let name = Box::leak(CString::new(name).unwrap().into_boxed_c_str());
        let description = Box::leak(CString::new(description).unwrap().into_boxed_c_str());
        let cmd = unsafe { XPLMCreateCommand(name.as_ptr(), description.as_ptr()) };
        unsafe { XPLMRegisterCommandHandler(cmd, handler, 1, refcon) };
        OwnedCommand {
            cmd,
            handler,
            refcon,
        }
    }
}

impl Drop for OwnedCommand {
    fn drop(&mut self) {
        unsafe { XPLMUnregisterCommandHandler(self.cmd, self.handler, 1, self.refcon) };
    }
}

/// Register all custom commands. Dropping the returned `Vec` unregisters every
/// handler, so the caller must keep it alive for the plugin's lifetime.
#[must_use]
pub fn register_commands(config: &PluginConfig) -> Vec<OwnedCommand> {
    *COMMAND_DATAREFS.lock().unwrap() = Some(CommandDataRefs::new());

    let mut commands = Vec::new();

    for (idx, &(_, name, desc)) in MODES.iter().enumerate() {
        commands.push(OwnedCommand::new(
            &format!("HoneycombBravo/{name}"),
            desc,
            Some(mode_command_handler),
            idx as *mut c_void,
        ));
    }

    commands.push(OwnedCommand::new(
        "HoneycombBravo/increase",
        "Increase the value of the autopilot mode selected with the rotary encoder.",
        Some(value_change_handler),
        REFCON_TRUE,
    ));
    commands.push(OwnedCommand::new(
        "HoneycombBravo/decrease",
        "Decrease the value of the autopilot mode selected with the rotary encoder.",
        Some(value_change_handler),
        REFCON_FALSE,
    ));

    commands.push(OwnedCommand::new(
        "HoneycombBravo/thrust_reversers",
        "Hold all thrust reversers on.",
        Some(reverser_handler),
        -1isize as *mut c_void,
    ));
    for i in 0..MAX_ENGINES {
        let n = i + 1;
        commands.push(OwnedCommand::new(
            &format!("HoneycombBravo/thrust_reverser_{n}"),
            &format!("Hold thrust reverser #{n} on."),
            Some(reverser_handler),
            i as *mut c_void,
        ));
    }

    register_trim_commands(config, &mut commands);

    xdebug!("Registered {} custom commands", commands.len());
    commands
}

fn register_trim_commands(config: &PluginConfig, commands: &mut Vec<OwnedCommand>) {
    let trim = &config.trim_wheel;
    if !trim.enabled {
        return;
    }

    if trim.detents_per_rotation <= 0.0 || trim.full_turns <= 0.0 {
        xdebug!(
            "Invalid trim wheel config (detents_per_rotation: {}, full_turns: {}), skipping trim registration",
            trim.detents_per_rotation,
            trim.full_turns
        );
        return;
    }

    let trim_dataref_name = CString::new(trim.elevator_trim_dataref.as_str()).unwrap();
    let trim_dataref = unsafe { XPLMFindDataRef(trim_dataref_name.as_ptr()) };
    if trim_dataref.is_null() {
        // Skip registration entirely; no-op commands would only hide the
        // misconfiguration in the joystick settings UI.
        xdebug!(
            "Could not find trim dataref '{}'; trim wheel commands not registered",
            trim.elevator_trim_dataref
        );
        return;
    }

    let trim_delta = (trim.max_trim - trim.min_trim) / trim.detents_per_rotation / trim.full_turns;
    *TRIM_STATE.lock().unwrap() = Some(TrimState {
        trim_dataref,
        trim_delta,
        min_trim: trim.min_trim,
        max_trim: trim.max_trim,
    });

    commands.push(OwnedCommand::new(
        "HoneycombBravo/elevator_trim_nose_up",
        "Trim elevator nose up",
        Some(trim_command_handler),
        REFCON_TRUE,
    ));
    commands.push(OwnedCommand::new(
        "HoneycombBravo/elevator_trim_nose_down",
        "Trim elevator nose down",
        Some(trim_command_handler),
        REFCON_FALSE,
    ));

    xdebug!(
        "Registered trim wheel commands (delta per detent: {:.6}, turns: {}, detents/rotation: {})",
        trim_delta,
        trim.full_turns,
        trim.detents_per_rotation
    );
}

/// Clear the static state populated by [`register_commands`], so a reload
/// cycle that keeps the dylib mapped doesn't leak stale pointers.
pub fn clear_command_state() {
    *COMMAND_DATAREFS.lock().unwrap() = None;
    *TRIM_STATE.lock().unwrap() = None;
}

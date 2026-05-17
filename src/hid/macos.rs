// Copyright (c) 2025 Jeremie Corbier
// SPDX-License-Identifier: GPL-3.0-or-later

//! macOS IOKit HID transport for Bravo LED feature reports.
//!
//! X-Plane already owns the Bravo for axes/buttons, so we never keep an open
//! handle: each LED update enumerates, opens non-exclusively, sends one
//! feature report and closes. The `IOHIDManager` is kept around for cheap
//! re-enumeration. See [`matching_dict`] for the device-identity match.

use crate::xdebug;
use core_foundation::base::{CFRelease, CFRetain, CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::set::CFSet;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation_sys::set::{CFSetGetCount, CFSetGetValues};
use std::ffi::c_void;
use std::time::{Duration, Instant};

// Honeycomb Bravo Throttle Quadrant.
const VENDOR_ID: i32 = 0x294B;
const PRODUCT_ID: i32 = 0x1901;

// HID top-level collection: Generic Desktop / Joystick.
// See <IOKit/hid/IOHIDUsageTables.h>.
const USAGE_PAGE_GENERIC_DESKTOP: i32 = 0x01;
const USAGE_JOYSTICK: i32 = 0x04;

const KERN_SUCCESS: IOReturn = 0;
const K_IO_HID_REPORT_TYPE_FEATURE: u32 = 2;
const K_IO_HID_OPTIONS_TYPE_NONE: u32 = 0;

/// Backoff applied after a string of failed opens or writes, to avoid hammering USB.
const FAILURE_BACKOFF: Duration = Duration::from_secs(2);
const MAX_CONSECUTIVE_FAILURES: u32 = 5;

type IOHIDManagerRef = *mut c_void;
type IOHIDDeviceRef = *mut c_void;
type IOReturn = i32;
type CFIndex = isize;
type CFSetRef = *const c_void;
type CFDictionaryRef = *const c_void;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOHIDManagerCreate(allocator: *const c_void, options: u32) -> IOHIDManagerRef;
    fn IOHIDManagerSetDeviceMatching(
        manager: IOHIDManagerRef,
        matching: CFDictionaryRef,
    ) -> IOReturn;
    fn IOHIDManagerOpen(manager: IOHIDManagerRef, options: u32) -> IOReturn;
    fn IOHIDManagerClose(manager: IOHIDManagerRef, options: u32);
    fn IOHIDManagerCopyDevices(manager: IOHIDManagerRef) -> CFSetRef;
    fn IOHIDDeviceOpen(device: IOHIDDeviceRef, options: u32) -> IOReturn;
    fn IOHIDDeviceClose(device: IOHIDDeviceRef, options: u32);
    fn IOHIDDeviceSetReport(
        device: IOHIDDeviceRef,
        report_type: u32,
        report_id: CFIndex,
        report: *const u8,
        report_length: CFIndex,
    ) -> IOReturn;
    fn IOHIDDeviceGetProperty(device: IOHIDDeviceRef, key: CFStringRef) -> CFTypeRef;
}

pub struct HidConnection {
    manager: Option<IOHIDManagerRef>,
    consecutive_failures: u32,
    backoff_until: Option<Instant>,
    /// Previous [`acquire_device`] result: drives the one-shot lost/acquired logs.
    had_device: bool,
    /// True between "entered backoff" and the next successful write, so the
    /// backoff sentence isn't re-logged every retry.
    backoff_logged: bool,
}

// SAFETY: X-Plane invokes plugin callbacks from a single thread; we never share
// the raw IOKit pointers across threads.
unsafe impl Send for HidConnection {}
unsafe impl Sync for HidConnection {}

impl HidConnection {
    pub fn new() -> Self {
        HidConnection {
            manager: None,
            consecutive_failures: 0,
            backoff_until: None,
            had_device: false,
            backoff_logged: false,
        }
    }

    /// Send a single feature report. `payload` is the report data only — no
    /// leading report-ID byte (IOKit takes the ID as a separate argument,
    /// unlike hidapi). Returns `true` on success.
    pub fn send_feature_report(&mut self, report_id: u8, payload: &[u8]) -> bool {
        if self.in_backoff() {
            return false;
        }

        let Some(device) = self.acquire_device() else {
            self.record_failure();
            return false;
        };

        let ok = unsafe {
            IOHIDDeviceSetReport(
                device,
                K_IO_HID_REPORT_TYPE_FEATURE,
                report_id as CFIndex,
                payload.as_ptr(),
                payload.len() as CFIndex,
            ) == KERN_SUCCESS
        };

        release_device(device);

        if ok {
            if self.backoff_logged {
                xdebug!("HID resumed from backoff");
                self.backoff_logged = false;
            }
            self.consecutive_failures = 0;
            self.backoff_until = None;
        } else {
            xdebug!("IOHIDDeviceSetReport failed");
            self.record_failure();
        }
        ok
    }

    fn in_backoff(&self) -> bool {
        self.backoff_until.is_some_and(|t| Instant::now() < t)
    }

    fn record_failure(&mut self) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
            if !self.backoff_logged {
                xdebug!(
                    "HID entering {}s backoff after {} consecutive failures",
                    FAILURE_BACKOFF.as_secs(),
                    MAX_CONSECUTIVE_FAILURES
                );
                self.backoff_logged = true;
            }
            self.backoff_until = Some(Instant::now() + FAILURE_BACKOFF);
            self.consecutive_failures = 0;
        }
    }

    fn acquire_device(&mut self) -> Option<IOHIDDeviceRef> {
        let manager = self.ensure_manager()?;

        unsafe {
            let device_set = IOHIDManagerCopyDevices(manager);
            if device_set.is_null() {
                self.note_device_lost();
                return None;
            }
            let set = CFSet::wrap_under_create_rule(device_set.cast());

            for device in devices_in_set(&set) {
                if IOHIDDeviceOpen(device, K_IO_HID_OPTIONS_TYPE_NONE) != KERN_SUCCESS {
                    continue;
                }
                self.note_device_acquired(device);
                CFRetain(device.cast_const());
                return Some(device);
            }
            self.note_device_lost();
            None
        }
    }

    /// One-shot log on the lost → acquired edge. Includes `LocationID` so a
    /// port / hub change is visible at a glance.
    fn note_device_acquired(&mut self, device: IOHIDDeviceRef) {
        if self.had_device {
            return;
        }
        let product = string_property(device, "Product").unwrap_or_else(|| "?".into());
        let serial = string_property(device, "SerialNumber").unwrap_or_else(|| "?".into());
        let location = number_property(device, "LocationID")
            .map(|n| format!("0x{:08X}", n as u32))
            .unwrap_or_else(|| "?".into());
        xdebug!(
            "HID acquired: {} (serial {}) at LocationID {} [VID:0x{:04X} PID:0x{:04X}]",
            product,
            serial,
            location,
            VENDOR_ID,
            PRODUCT_ID
        );
        self.had_device = true;
    }

    /// One-shot log on the acquired → lost edge. Silent during long disconnects.
    fn note_device_lost(&mut self) {
        if !self.had_device {
            return;
        }
        xdebug!("HID lost: no matching Bravo on the bus (was previously acquired)");
        self.had_device = false;
    }

    fn ensure_manager(&mut self) -> Option<IOHIDManagerRef> {
        if let Some(m) = self.manager {
            return Some(m);
        }

        unsafe {
            let manager = IOHIDManagerCreate(std::ptr::null(), K_IO_HID_OPTIONS_TYPE_NONE);
            if manager.is_null() {
                xdebug!("IOHIDManagerCreate failed");
                return None;
            }

            let matching = matching_dict();
            if IOHIDManagerSetDeviceMatching(manager, matching.as_concrete_TypeRef().cast())
                != KERN_SUCCESS
            {
                xdebug!("IOHIDManagerSetDeviceMatching failed");
                IOHIDManagerClose(manager, K_IO_HID_OPTIONS_TYPE_NONE);
                CFRelease(manager.cast_const());
                return None;
            }

            if IOHIDManagerOpen(manager, K_IO_HID_OPTIONS_TYPE_NONE) != KERN_SUCCESS {
                xdebug!("IOHIDManagerOpen failed");
                IOHIDManagerClose(manager, K_IO_HID_OPTIONS_TYPE_NONE);
                CFRelease(manager.cast_const());
                return None;
            }

            self.manager = Some(manager);
            Some(manager)
        }
    }
}

impl Drop for HidConnection {
    fn drop(&mut self) {
        if let Some(manager) = self.manager.take() {
            unsafe {
                IOHIDManagerClose(manager, K_IO_HID_OPTIONS_TYPE_NONE);
                CFRelease(manager.cast_const());
            }
        }
    }
}

fn release_device(device: IOHIDDeviceRef) {
    unsafe {
        IOHIDDeviceClose(device, K_IO_HID_OPTIONS_TYPE_NONE);
        CFRelease(device.cast_const());
    }
}

/// `IOHIDManager` matching dictionary. Pins all four properties so we only ever
/// see the Bravo's joystick top-level collection — never the Alpha yoke
/// (`0x294B:0x1900`) or any other HID collection on the same bus:
///
///   * `VendorID`        = `0x294B` (Honeycomb)
///   * `ProductID`       = `0x1901` (Bravo)
///   * `DeviceUsagePage` = `0x01`   (Generic Desktop)
///   * `DeviceUsage`     = `0x04`   (Joystick)
fn matching_dict() -> CFDictionary<CFString, CFNumber> {
    CFDictionary::from_CFType_pairs(&[
        (
            CFString::from_static_string("VendorID"),
            CFNumber::from(VENDOR_ID),
        ),
        (
            CFString::from_static_string("ProductID"),
            CFNumber::from(PRODUCT_ID),
        ),
        (
            CFString::from_static_string("DeviceUsagePage"),
            CFNumber::from(USAGE_PAGE_GENERIC_DESKTOP),
        ),
        (
            CFString::from_static_string("DeviceUsage"),
            CFNumber::from(USAGE_JOYSTICK),
        ),
    ])
}

fn devices_in_set(set: &CFSet) -> Vec<IOHIDDeviceRef> {
    unsafe {
        let set_ref = set.as_concrete_TypeRef();
        let count = CFSetGetCount(set_ref) as usize;
        if count == 0 {
            return Vec::new();
        }

        let mut values: Vec<*const c_void> = vec![std::ptr::null(); count];
        CFSetGetValues(set_ref, values.as_mut_ptr());
        values
            .into_iter()
            .filter(|p| !p.is_null())
            .map(|p| p.cast_mut())
            .collect()
    }
}

fn string_property(device: IOHIDDeviceRef, key: &'static str) -> Option<String> {
    unsafe {
        let key = CFString::from_static_string(key);
        let value = IOHIDDeviceGetProperty(device, key.as_concrete_TypeRef());
        if value.is_null() {
            return None;
        }
        let cf_type = CFType::wrap_under_get_rule(value);
        cf_type.downcast::<CFString>().map(|s| s.to_string())
    }
}

fn number_property(device: IOHIDDeviceRef, key: &'static str) -> Option<i64> {
    unsafe {
        let key = CFString::from_static_string(key);
        let value = IOHIDDeviceGetProperty(device, key.as_concrete_TypeRef());
        if value.is_null() {
            return None;
        }
        let cf_type = CFType::wrap_under_get_rule(value);
        cf_type.downcast::<CFNumber>().and_then(|n| n.to_i64())
    }
}

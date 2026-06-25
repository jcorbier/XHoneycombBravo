# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-06-25

### Fixed
- **ANTI ICE annunciator no longer inverted.** Follows `sim/cockpit2/annunciators/pitot_heat` directly: lit when X-Plane thinks anti-ice is required and not active.
- **PARKING BRAKE annunciator wired to the parking-brake dataref** (`sim/cockpit2/controls/parking_brake_ratio`) instead of the toe-brake one (`wheel_brake_ratio`). The old `wheel_brake` config key still loads via `serde(alias)`.
- **Gear-deployed threshold.** Green/red logic uses `>= 0.99` / `<= 0.01` so aircraft that report `0.9999` for fully-extended gear show green instead of red.
- **Trim wheel no longer registers no-op commands** when its dataref is missing (used to register handlers and log a warning).
- **Preferences directory looked up via `XPLMGetPrefsPath`** instead of walking the current working directory looking for a `Resources` folder. The old probe failed whenever X-Plane was launched from somewhere unusual.

### Changed
- **Internal names match the panel silkscreen** (`master_warning`, `low_oil_pressure`, `starter_engaged`, `parking_brake`, …). Pre-rename config keys still load via `serde(alias)`.
- **Native macOS IOKit replaces `hidapi` and `rusb`** for LED feature reports; no Homebrew dependency.
- **LED writes are ephemeral** (open / send / close per update) and non-exclusive, so X-Plane keeps the joystick handle. No startup delay, no `hid_read_value_timeout` errors.
- **Device matching pinned** to VID `0x294B` / PID `0x1901` / Usage Page `0x01` / Usage `0x04`, so the Alpha yoke or other HID collections can't be picked up by mistake.
- **Flight loop throttled to 20 Hz** (50 ms); LEDs feel instant and we stop burning ~5× the dataref reads.
- **3 s forced LED refresh** on top of the existing change-cache, so silent hardware resets (USB unplug/replug, sleep/wake) re-sync automatically instead of staying dark until the next dataref change.
- **Rust 2024 edition.** Dependencies bumped, `once_cell` replaced with stdlib `OnceLock` / `LazyLock`.
- **Command handlers wrapped in `OwnedCommand` RAII** so `XPluginStop` unregisters every handler before the dylib unloads.
- **Cleanup pass.** Single source of truth for LED panel labels (`ALL_LEDS` in `hid`) and for `mode_*` commands (`MODES` in `commands`); `Option<&T>` dataref accessors; pedantic-clippy clean.
- **Co-authored.** [Jonas Lalin](https://github.com/jonaslalin/) added as `Cargo.toml` co-author for the post-0.3.0 work.

### Added
- **`[system] leds_enabled`** config option (default `true`). Set to `false` to skip all HID LED access.
- **Plugin version printed on startup.** First log line: `Plugin starting v<version> (built for Rust <toolchain>)`.
- **Sim-elapsed timestamp on every plugin log line** (`t=12.345s`), via `XPLMGetElapsedTime`, since `XPLMDebugString` is not timestamped.
- **Diagnostic LED logging.** One line on every change to the desired LED state, listing the lit lamps and every dataref that fed into the decision.
- **HID lifecycle logging.** One-shot `HID acquired` / `HID lost` / `HID entering Ns backoff` / `HID resumed from backoff` lines on the edges, so hot-plug and port-change events are obvious.
- **Rotary encoder turn logging** for confirming joystick bindings reach the plugin.
- **GitHub Actions CI** running `cargo fmt --check`, `cargo clippy -D warnings`, and a universal release build.

## [0.3.0] - 2026-02-09

### Added
- Configurable trim wheel support with custom X-Plane commands (`HoneycombBravo/elevator_trim_nose_up` and `HoneycombBravo/elevator_trim_nose_down`), thanks to [Jonas Lalin](https://github.com/jonaslalin/).
- New `[trim_wheel]` section in configuration file with adjustable parameters: `full_turns`, `detents_per_rotation`, `min_trim`, `max_trim`, and `elevator_trim_dataref`.

### Changed
- Renamed `LedConfig` to `PluginConfig` to reflect the broader scope of the configuration.
- Updated plugin description to cover all features (LEDs, rotary encoder, trim wheel, thrust reversers).

## [0.2.1] - 2026-01-07

### Added
- Skunkcrafts updater support.

## [0.2.0] - 2025-12-23

### Added
- Hotplug support: The plugin now detects if the Honeycomb Bravo is connected after the plugin has started.
- Hybrid device detection: Uses `libusb` (via `rusb`) for immediate hotplug events where available, with a fallback to polling.
- Disconnection detection: Automatically detects if the device is unplugged or encounters a communication error and resets state for reconnection.

### Changed
- Internal logging now uses a consistent `xdebug!` macro with "XHoneycombBravo |" prefix.
- Refactored HID layer to separate detection logic from communication logic.

## [0.1.0] - 2025-12-20

### Added
- Initial release of X-HoneycombBravo plugin.
- LED control support for:
  - Autopilot buttons (HDG, NAV, APR, REV, ALT, VS, IAS, AP).
  - Landing gear indicators.
  - Annunciator panel (Warning, Caution, Fire, Oil, Fuel, etc.).
- Custom commands for the rotary knob (IAS, CRS, HDG, VS, ALT) and thrust reversers.
- Configuration file support (`XHoneycombBravo.cfg`) in X-Plane preferences.
- macOS native binary generation (Universal `x86_64` and `arm64`).

[Unreleased]: https://github.com/jcorbier/XHoneycombBravo/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/jcorbier/XHoneycombBravo/releases/tag/v0.4.0
[0.3.0]: https://github.com/jcorbier/XHoneycombBravo/releases/tag/v0.3.0
[0.2.1]: https://github.com/jcorbier/XHoneycombBravo/releases/tag/v0.2.1
[0.2.0]: https://github.com/jcorbier/XHoneycombBravo/releases/tag/v0.2.0
[0.1.0]: https://github.com/jcorbier/XHoneycombBravo/releases/tag/v0.1.0

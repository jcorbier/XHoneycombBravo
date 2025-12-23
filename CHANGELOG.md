# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/jcorbier/XHoneycombBravo/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jcorbier/XHoneycombBravo/releases/tag/v0.1.0

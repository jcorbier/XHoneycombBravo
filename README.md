# Honeycomb Bravo X-Plane 12 Plugin

A native X-Plane 12 plugin written in Rust that controls the Honeycomb Bravo throttle quadrant LEDs based on aircraft systems and datarefs. This is a Rust port of the original FlyWithLua script.

## Features

- **Autopilot LEDs**: HDG, NAV, APR, REV, ALT, VS, IAS, and AP status indicators
- **Landing Gear LEDs**: Green (deployed), Red (moving), Off (stowed) for all three gear
- **Annunciator Panel**: 14 warning and caution lights including:
  - Master Warning/Caution
  - Engine Fire
  - Low Oil/Fuel Pressure
  - Anti-Ice
  - Starter Engaged
  - APU Running
  - Vacuum, Hydraulic, Voltage warnings
  - Parking Brake, Door indicators, and more
- **Rotary Encoder Support**: Control IAS, CRS, HDG, VS, ALT, and COM1 frequencies
- **Thrust Reverser Commands**: Individual engine or all-engine thrust reverser control

## System Requirements

- X-Plane 12
- macOS (Apple Silicon or Intel)
- Honeycomb Bravo Throttle Quadrant
- hidapi library (install via Homebrew: `brew install hidapi`)

## Building from Source

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Install hidapi:
   ```bash
   brew install hidapi
   ```
3. Clone this repository and build:
   ```bash
   cd /Users/jcorbier/Code/XHoneycombBravo
   cargo build --release
   ```
4. The compiled plugin will be at `target/release/libhoneycomb_bravo_xplane.dylib`

## Installation

1. Build the plugin (see above)
2. Create the plugin directory structure in X-Plane:
   ```
   X-Plane 12/Resources/plugins/HoneycombBravo/mac_x64/
   ```
3. Copy the compiled `.dylib` file to the `mac_x64` directory
4. Rename it to `mac.xpl`:
   ```bash
   cp target/release/libhoneycomb_bravo_xplane.dylib \
      "~/X-Plane 12/Resources/plugins/HoneycombBravo/mac_x64/mac.xpl"
   ```
5. Restart X-Plane 12

## Custom Commands

The plugin registers the following custom commands that can be mapped to the Honeycomb Bravo buttons:

### Autopilot Mode Selection
- `HoneycombBravo/mode_ias` - Set rotary encoder to IAS mode
- `HoneycombBravo/mode_crs` - Set rotary encoder to CRS mode
- `HoneycombBravo/mode_hdg` - Set rotary encoder to HDG mode
- `HoneycombBravo/mode_vs` - Set rotary encoder to VS mode
- `HoneycombBravo/mode_alt` - Set rotary encoder to ALT mode
- `HoneycombBravo/mode_com1_coarse` - Set rotary encoder to COM1 coarse tuning
- `HoneycombBravo/mode_com1_fine` - Set rotary encoder to COM1 fine tuning

### Rotary Encoder
- `HoneycombBravo/increase` - Increase the selected autopilot value
- `HoneycombBravo/decrease` - Decrease the selected autopilot value

### Thrust Reversers
- `HoneycombBravo/thrust_reversers` - Hold all thrust reversers on
- `HoneycombBravo/thrust_reverser_1` through `thrust_reverser_8` - Individual engine reversers

## Configuration

### LED Mappings

The plugin supports custom LED-to-dataref mappings via a configuration file. On first run, the plugin will create a default configuration file at:

```
X-Plane 12/Output/preferences/XHoneycombBravo.cfg
```

You can edit this TOML file to customize which datarefs control which LEDs. For example:

```toml
[autopilot]
hdg = "sim/cockpit2/autopilot/heading_mode"
nav = "sim/cockpit2/autopilot/nav_status"
# ... customize other autopilot datarefs

[annunciators]
master_warning = "sim/cockpit2/annunciators/master_warning"
engine_fire = "sim/cockpit2/annunciators/engine_fires"
# ... customize annunciator datarefs
```

Changes to the configuration file require restarting X-Plane or reloading the plugin.

### Joystick Mapping

Map the Honeycomb Bravo's rotary encoder and buttons to the custom commands in X-Plane's joystick settings (Settings > Joystick > Buttons: Advanced).

## License

MIT License - see LICENSE file for details

## Credits

Based on the original HoneycombBravoMacHelper FlyWithLua script, which was itself based on HoneycombBravoHelper for Linux by Daniel Peukert.

Modified for macOS by Joe Milligan, ported to Rust by Jeremie Corbier.

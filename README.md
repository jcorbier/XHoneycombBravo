# Honeycomb Bravo X-Plane 12 Plugin

A native X-Plane 12 plugin written in Rust for the Honeycomb Bravo throttle quadrant. Controls LEDs based on aircraft datarefs, provides rotary encoder autopilot control, configurable trim wheel support, and thrust reverser commands. Originally a Rust port of the FlyWithLua script, now extended with additional features.

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
- **Trim Wheel**: Configurable elevator trim with adjustable sensitivity (turns, detents per rotation, trim range)
- **Thrust Reverser Commands**: Individual engine or all-engine thrust reverser control

## System Requirements

- X-Plane 12
- macOS (Apple Silicon or Intel)
- Honeycomb Bravo Throttle Quadrant
- hidapi library (install via Homebrew: `brew install hidapi`)

## Installation

1. Download the latest release `XHoneycombBravo-vX.Y.Z.zip` from the [Releases page](../../releases).
2. Unzip the file. You will get a folder named `XHoneycombBravo`.
3. Copy the `XHoneycombBravo` folder to your X-Plane plugins directory:
   ```
   X-Plane 12/Resources/plugins/
   ```

### Important: Security Quarantine

Since this plugin is not yet notarized by Apple, macOS will block it from running by default. To allow it:

1. Open a terminal.
2. Run the following command to remove the quarantine attribute from the plugin:
   ```bash
   xattr -cr "~/X-Plane 12/Resources/plugins/XHoneycombBravo"
   ```
   (Adjust the path if your X-Plane installation is in a different location)

Alternatively, you can try opening the plugin file manually via Finder (Right-click > Open) to trigger the security exception dialogue, but the `xattr` method is more reliable.

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

### Trim Wheel
- `HoneycombBravo/elevator_trim_nose_up` - Trim elevator nose up
- `HoneycombBravo/elevator_trim_nose_down` - Trim elevator nose down

### Thrust Reversers
- `HoneycombBravo/thrust_reversers` - Hold all thrust reversers on
- `HoneycombBravo/thrust_reverser_1` through `thrust_reverser_8` - Individual engine reversers

## Configuration

The plugin uses a TOML configuration file. On first run, it creates a default at:

```
X-Plane 12/Output/preferences/XHoneycombBravo.cfg
```

Changes to the configuration file require restarting X-Plane or reloading the plugin.

### LED Mappings

Customize which datarefs control which LEDs:

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

### Trim Wheel

The trim wheel sends 24 detent commands per full 360° rotation. The plugin calculates the trim delta per detent as:

```
delta = (max_trim - min_trim) / detents_per_rotation / full_turns
```

Default configuration (Cessna 172):

```toml
[trim_wheel]
enabled = true
elevator_trim_dataref = "sim/cockpit2/controls/elevator_trim"
min_trim = -1.0
max_trim = 1.0
full_turns = 10.0
detents_per_rotation = 24.0
```

Adjust `full_turns` to match your aircraft (e.g., a Cessna 172 has ~10 full turns from max nose down to max nose up). For add-on aircraft that use a different trim dataref, change `elevator_trim_dataref` accordingly.

### Joystick Mapping

Map the Honeycomb Bravo's controls to the custom commands in X-Plane's joystick settings (Settings > Joystick > Buttons: Advanced):

1. **Rotary encoder**: Bind to `HoneycombBravo/increase` and `HoneycombBravo/decrease`
2. **Mode buttons**: Bind to the `HoneycombBravo/mode_*` commands
3. **Trim wheel**: Find the trim wheel up/down buttons and rebind them to `HoneycombBravo/elevator_trim_nose_up` and `HoneycombBravo/elevator_trim_nose_down`
4. **Thrust reversers**: Bind to `HoneycombBravo/thrust_reversers` or individual engine commands

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

### Installing from Source

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

## License

GPLv3 License - see LICENSE file for details

## Credits

Based on the original HoneycombBravoMacHelper FlyWithLua script, which was itself based on HoneycombBravoHelper for Linux by Daniel Peukert.

Modified for macOS by Joe Milligan, ported to Rust by Jeremie Corbier. Trim wheel support added by [Jonas Lalin](https://github.com/jonaslalin/), based on the [HoneycombBravoTrimHelper](https://gist.github.com/Spo1ler/fa89eec64fdae462adf7a0a53c19987b) FlyWithLua script by Egor Shkorov.

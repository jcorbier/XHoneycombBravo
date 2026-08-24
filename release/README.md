# Honeycomb Bravo X-Plane 12 Plugin

Native X-Plane 12 plugin in Rust for the Honeycomb Bravo throttle quadrant. Drives the panel LEDs from aircraft datarefs and registers custom commands for the rotary encoder, trim wheel, and thrust reversers.

Originally a Rust port of the FlyWithLua scripts (see [Credits](#credits)), now extended with native macOS IOKit HID, trim wheel support, and a diagnostic log layer.

## Features

- **Autopilot LEDs**: HDG, NAV, APR, REV, ALT, VS, IAS, AP
- **Landing gear LEDs**: green (deployed), red (in transit), off (stowed)
- **All 14 annunciators**: MASTER WARNING / CAUTION, ENGINE FIRE, LOW OIL / FUEL / HYD PRESSURE, LOW VOLTS, ANTI ICE, STARTER ENGAGED, APU, VACUUM, AUX FUEL PUMP, PARKING BRAKE, DOOR
- **Rotary encoder commands**: IAS, CRS, HDG, VS, ALT, COM1 coarse / fine
- **Trim wheel**: configurable elevator trim (turns, detents per rotation, trim range)
- **Thrust reversers**: per-engine and all-engine commands

## Requirements

- X-Plane 12
- macOS (Apple Silicon or Intel)
- Honeycomb Bravo Throttle Quadrant

No Homebrew libraries needed: LED control uses native macOS IOKit.

## Installation

1. Download the latest `XHoneycombBravo-vX.Y.Z.zip` from the [Releases page](../../releases).
2. Unzip and drop the resulting `XHoneycombBravo` folder into `X-Plane 12/Resources/plugins/`.
3. Clear the macOS quarantine attribute (the plugin is not Apple-notarized):
   ```bash
   xattr -cr "$HOME/X-Plane 12/Resources/plugins/XHoneycombBravo"
   ```
   `~` does **not** expand inside double quotes in zsh/bash, so use `"$HOME/..."` or put the tilde outside the quotes (`xattr -cr ~/"X-Plane 12/Resources/plugins/XHoneycombBravo"`). Adjust the path if your X-Plane install lives elsewhere.
4. Launch X-Plane 12.

## Custom Commands

All commands live under the `HoneycombBravo/` prefix and can be bound from **Settings → Joystick → Buttons: Advanced** in X-Plane.

| Group              | Command                                                                 | What it does                          |
| ------------------ | ----------------------------------------------------------------------- | ------------------------------------- |
| Rotary mode        | `mode_ias`, `mode_crs`, `mode_hdg`, `mode_vs`, `mode_alt`               | Select which value the encoder edits  |
| Rotary mode        | `mode_com1_coarse`, `mode_com1_fine`                                    | Tune COM1 standby with the encoder    |
| Rotary value       | `increase`, `decrease`                                                  | Step the selected value up / down     |
| Trim wheel         | `elevator_trim_nose_up`, `elevator_trim_nose_down`                      | One trim detent in the given direction |
| Thrust reversers   | `thrust_reversers`                                                      | Hold reversers on for all engines     |
| Thrust reversers   | `thrust_reverser_1` … `thrust_reverser_8`                               | Hold reverser on for engine N         |

## Configuration

A TOML config is generated on first run at `X-Plane 12/Output/preferences/XHoneycombBravo.cfg`. Edit the file and restart X-Plane to apply changes.

### LED mappings

Override which datarefs drive which LEDs (defaults target stock X-Plane aircraft):

```toml
[autopilot]
hdg = "sim/cockpit2/autopilot/heading_mode"
nav = "sim/cockpit2/autopilot/nav_status"
# ...

[annunciators]
master_warning = "sim/cockpit2/annunciators/master_warning"
engine_fire = "sim/cockpit2/annunciators/engine_fires"
# ...
```

### LED HID access

```toml
[system]
leds_enabled = true
```

With `leds_enabled = true` (the default) LED updates go out over IOKit with non-exclusive access (`kIOHIDOptionsTypeNone`). The plugin opens the device only for the duration of one feature-report write and closes immediately, so X-Plane keeps the joystick handle. Set it to `false` to disable all HID traffic and use only the rotary encoder / trim wheel commands.

### Trim wheel

The Bravo wheel emits 24 detent events per 360° turn. Each detent adjusts the trim by:

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

Tweak `full_turns` to match your aircraft (a Cessna 172 takes ~10 turns from full nose-down to full nose-up). Override `elevator_trim_dataref` for add-ons that use their own trim dataref.

## Building from source

1. Install Rust via [rustup.rs](https://rustup.rs/).
2. Build a universal binary (Apple Silicon + Intel):
   ```bash
   rustup target add aarch64-apple-darwin x86_64-apple-darwin
   cargo build --release --target aarch64-apple-darwin
   cargo build --release --target x86_64-apple-darwin
   lipo -create -output mac.xpl \
       target/aarch64-apple-darwin/release/libhoneycomb_bravo_xplane.dylib \
       target/x86_64-apple-darwin/release/libhoneycomb_bravo_xplane.dylib
   ```
3. Install and clear quarantine:
   ```bash
   mkdir -p "$HOME/X-Plane 12/Resources/plugins/XHoneycombBravo"
   mv mac.xpl "$HOME/X-Plane 12/Resources/plugins/XHoneycombBravo/mac.xpl"
   xattr -cr "$HOME/X-Plane 12/Resources/plugins/XHoneycombBravo"
   ```
4. Restart X-Plane 12.

For a fast single-arch dev build (current Mac only): `cargo build --release` produces `target/release/libhoneycomb_bravo_xplane.dylib`.

## Troubleshooting

Plugin output lands in `X-Plane 12/Log.txt`, prefixed with `XHoneycombBravo |` and tagged with `t=<sim_seconds>` so it lines up with X-Plane's own log entries:

```bash
grep "XHoneycombBravo" "$HOME/X-Plane 12/Log.txt" | tail -50
```

### LED state transitions

One line per change to the desired LED state, listing the lit lamps and every dataref the decision read. Quiet during steady flight, immediately useful when a lamp misbehaves:

```
XHoneycombBravo | t=12.345s | LED change | pwr=1 bus=13.27V | ON=[HDG+AP+L_GREEN+N_GREEN+R_GREEN+VACUUM+PARKING_BRAKE] | banks=[81 15 40 02] | dr: ap=1 hdg=1 nav=0 ...
```

`pwr=0` means X-Plane reports zero bus voltage; check the master / battery switch. The `dr:` section is exactly what the plugin read, so it pinpoints whether a wrong lamp is the plugin, a `XHoneycombBravo.cfg` mapping, or the aircraft itself.

### USB and hot-plug events

HID lifecycle transitions are logged on the edges only, not per tick:

```
XHoneycombBravo | HID acquired: Bravo Throttle Quadrant (serial 65AB...) at LocationID 0x20430000 [VID:0x294B PID:0x1901]
XHoneycombBravo | HID lost: no matching Bravo on the bus (was previously acquired)
XHoneycombBravo | HID entering 2s backoff after 5 consecutive failures
XHoneycombBravo | HID resumed from backoff
```

If the panel is dark but X-Plane still sees the Bravo as a joystick, the LED microcontroller is wedged. Unplug the USB cable for 15+ seconds and replug; the plugin re-acquires automatically.

### Custom-command activity

Mode buttons and the encoder log one line each, so you can confirm joystick bindings reach the plugin:

```
XHoneycombBravo | Rotary mode -> Hdg
XHoneycombBravo | Rotary turn: dir=+ mode=Hdg
```

If nothing appears, the binding under **Settings → Joystick** is wrong or points at a stock X-Plane command instead of `HoneycombBravo/mode_*`.

## License

GPL-3.0. See [LICENSE](LICENSE).

## Credits

- Original Linux helper: HoneycombBravoHelper by Daniel Peukert.
- macOS FlyWithLua port: HoneycombBravoMacHelper by Joe Milligan.
- Rust port: Jeremie Corbier.
- Trim wheel support: [Jonas Lalin](https://github.com/jonaslalin/), based on the [HoneycombBravoTrimHelper](https://gist.github.com/Spo1ler/fa89eec64fdae462adf7a0a53c19987b) FlyWithLua script by Egor Shkorov.

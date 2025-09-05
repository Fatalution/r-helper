# R-Helper v0.3.0

A Windows application for controlling Razer Blade settings w/o Synapse.

## Features

- Performance modes: Battery, Silent, Balanced, Performance, Hyperboost
	- Custom is shown disabled; if active externally, it appears muted-green but remains non-clickable
- Fan control: Auto/Manual, with current RPM display
- Keyboard backlight: Brightness control
- Logo lighting: Static, Breathing, Off
- Battery care: Toggle charging threshold (80%)


## Installation

1. Download the latest release
2. Extract anywhere
3. Run `rhelper.exe`

## Building

```powershell
cargo build --release
```

## Architecture

Core device control via locally vendored `librazer` (derived from razer-ctl)


## License

MIT. Includes MIT-licensed portions derived from razer-ctl (see NOTICE and THIRD_PARTY_LICENSES.md).

## Support

If you really want to express gratitude: [PayPal Donation](https://www.paypal.com/paypalme/fatalutionDE)

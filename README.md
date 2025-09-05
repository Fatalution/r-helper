R-Helper — Razer Blade control UI (Windows)

Overview
- Lightweight egui app to toggle performance modes, fans, logo lighting, and battery care on Razer Blade laptops.
- Uses a locally vendored `librazer` library derived from blauzim/razer-ctl.
- No GPU management; no UAC elevation.

Build
- Prereqs: Rust stable (MSVC toolchain) on Windows.
- Build: `cargo build --release`
- Run: `target/release/rhelper.exe`

Notes
- App icon is embedded via `razer-gui.rc` and `rhelper.ico`.
- No admin required.
- Custom mode is shown disabled; if active externally (e.g., via Synapse), it appears muted-green but remains non-clickable.

Supported
- Tested primarily on Razer Blade 16 (2025), should work on other Blades that expose similar HID features.

Credits
- Based on and inspired by: https://github.com/blauzim/razer-ctl (MIT)

License
- MIT. See LICENSE. See NOTICE and THIRD_PARTY_LICENSES.md for attributions.

Disclaimer
- “Razer” is a trademark of Razer Inc. This project is not affiliated with or endorsed by Razer.

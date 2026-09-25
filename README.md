# locsim

<div align="center">

[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-orange.svg?style=for-the-badge&logo=rust)](https://crates.io/crates/locsim)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg?style=for-the-badge)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-blueviolet.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Platform Support](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux%20%7C%20macOS%20%7C%20Android%20%7C%20iOS-brightgreen.svg?style=for-the-badge)](https://github.com/itznan/locsim)
[![Tests Passing](https://img.shields.io/badge/tests-46%20passed-success.svg?style=for-the-badge&logo=githubactions)](https://github.com/itznan/locsim)

**A universal, cross-platform location simulator written in 100% safe Rust.**  
*Instantly spoof device coordinates, altitude, accuracy, speed, and heading across Windows, Linux, macOS, Android emulators, iOS simulators, and headless browsers.*

[Features](#key-features) • [Installation](#installation) • [Quick Start](#quick-start-in-10-seconds) • [Usage Guide](#complete-usage-guide) • [Docs](#deep-dive-documentation) • [CI/CD Testing](#automated-testing--cicd)

</div>

---

## Why `locsim`?

Testing location-dependent applications is notoriously fragmented:
- **Browsers** require Chrome DevTools Protocol (`Emulation.setGeolocationOverride`) or manual sensor panels.
- **Android emulators** require `adb emu geo fix` or telnet commands with custom parameter ordering.
- **iOS simulators** require `xcrun simctl location booted set`.
- **Desktop operating systems** require registry overrides, GeoClue static configs, or NMEA-0183 serial feeds.
- **CI/CD test suites** require deterministic JSON payloads without third-party rate limits.

**`locsim` solves this with a single command.**  
Whether you provide a place name (`"Tokyo Station"`), coordinates (`35.6812, 139.7671`), or a saved bookmark (`locsim use home`), `locsim` resolves the position, validates all coordinates and telemetry bounds, and simultaneously broadcasts the simulated state to your operating system, connected mobile emulators, browser test harnesses, and developer test scripts.

---

## Multi-Layer Architecture

```text
                               ┌──────────────────────────┐
                               │   locsim CLI / CI Test   │
                               └────────────┬─────────────┘
                                            │
                                            ▼
                    ┌───────────────────────────────────────────────┐
                    │               Location Core                   │
                    │  - Lat / Lon Validation (-90..90, -180..180)  │
                    │  - Altitude, Accuracy, Speed, Heading         │
                    │  - In-Memory & Disk Cached Geocoding          │
                    └───────────────────────┬───────────────────────┘
                                            │
         ┌──────────────────┬───────────────┼───────────────┬──────────────────┐
         ▼                  ▼               ▼               ▼                  ▼
  ┌──────────────┐   ┌──────────────┐ ┌───────────┐  ┌─────────────┐    ┌──────────────┐
  │   Windows    │   │    Linux     │ │   macOS   │  │   Android   │    │  Chromium    │
  │ WinRT / HKCU │   │  GeoClue2 &  │ │   Xcode   │  │  ADB Bridge │    │  CDP Sensors │
  │ & Sensor HW  │   │  gpsd NMEA   │ │  simctl   │  │ emu geo fix │    │ & Playwright │
  └──────────────┘   └──────────────┘ └───────────┘  └─────────────┘    └──────────────┘
```

---

## Key Features

- **Natural Language Geocoding**: Resolve any address or point of interest with OpenStreetMap Nominatim, LocationIQ, Mapbox, OpenCage, or Google Maps.
- **First-Class Reverse Geocoding**: Convert coordinates into human-readable street addresses instantly via `--reverse` / `-r`.
- **Extended Real-World Telemetry**: Full simulation of **altitude** (meters), horizontal **accuracy** (meters), ground **speed** (m/s), and **heading** (0.0°–360.0° bearing).
- **Mobile Emulator Bridges**:
  - **Android ADB**: Automatically detects running AVDs and dispatches `adb emu geo fix` and `cmd location set-location`. Target individual devices with `--android-device`.
  - **iOS Simulator**: Automatic synchronization via `xcrun simctl location booted set`.
- **Named Location Profiles / Bookmarks**: Save, list, apply, and delete frequently used testing spots (`locsim save home`, `locsim use work`).
- **Reset with IP Geolocation**: Instantly clear all mock states and detect genuine physical location via public IP lookup (`locsim reset`).
- **Browser & Automation Exports**: Automatically updates Chrome DevTools Protocol (`cdp_geolocation.json`), GPX 1.1 tracks (`simulated_location.gpx`), and shell environment scripts (`locsim_env.sh` / `locsim_env.ps1`).
- **Shell Auto-Completions**: First-class shell completions for PowerShell, Bash, Zsh, Fish, and Elvish.
- **Modern TOML Configuration**: Global defaults in `~/.config/locsim/config.toml` with per-project `.locsimrc` overrides.
- **Zero Unsafe Code**: 100% safe Rust, thread-safe parallel test execution, and comprehensive error handling.

---

## Installation

### Via Cargo (Recommended)

```bash
cargo install locsim
```

### Building from Source

```bash
git clone https://github.com/itznan/locsim.git
cd locsim
cargo build --release
```

The compiled binary will be located in:
- `target/release/locsim` (Linux / macOS)
- `target/release/locsim.exe` (Windows)

---

## Quick Start in 10 Seconds

```bash
# 1. Search any location by name and apply (with auto-confirm -y)
locsim "Empire State Building, New York" -y

# 2. View active simulation status, diagnostics, and exports
locsim --show

# 3. Save as a bookmark for rapid testing
locsim save nyc

# 4. Jump anywhere else, then switch back instantly
locsim "Tokyo Tower" -y
locsim use nyc

# 5. Clear simulation and detect your real physical IP location
locsim reset
```

---

## Complete Usage Guide


### 1. Interactive Default Prompt
Simply run `locsim` without arguments to launch an interactive search session:

```text
$ locsim

╔══════════════════════════════════════╗
║       Location Simulator             ║
╚══════════════════════════════════════╝

Enter a location:
> Eiffel Tower, Paris, France

Searching location...

✓ Location found

Location : Eiffel Tower
Address  : 5, Avenue Anatole France, Quartier du Gros-Caillou, Paris, 75007, France

Latitude : 48.858370
Longitude: 2.294481

Set this as simulated location? [Y/n] > y

✓ Simulated location successfully applied!
  Provider: Windows Location Provider (WinRT / Sensor Driver Overrides / DevTools)
  OS Mechanism: WinRT Sensor Testing Overrides & Developer State

Generated integration files:
  • ~/.config/locsim/current_location.json
  • ~/.config/locsim/simulated_location.gpx
  • ~/.config/locsim/cdp_geolocation.json
  • ~/.config/locsim/locsim_env.ps1
```

### 2. Search by Place Name or Address
```bash
# Search with interactive confirmation
locsim "Sydney Opera House"

# Search and automatically accept confirmation (-y / --yes)
locsim "Berlin Hauptbahnhof" -y
```

### 3. First-Class Reverse Geocoding (`-r` / `--reverse`)
Translate GPS coordinates back into a human-readable street address:
```bash
# Lookup coordinates and display address
locsim -r 37.7749 -122.4194

# Lookup coordinates and apply immediately to OS & emulators
locsim -r 37.7749 -122.4194 -y
```

### 4. Manual Coordinate Input (`--lat` & `--lon`)
Directly set coordinates with strict boundary verification:
```bash
locsim --lat 22.3688 --lon 70.8022 -y
```

### 5. Extended Real-World Telemetry
Simulate realistic vehicular movement, flight, drone telemetry, or low-accuracy GPS drift:
```bash
locsim "Golden Gate Bridge" \
  --altitude 67.5 \
  --accuracy 5.0 \
  --speed 18.5 \
  --heading 350.0 \
  -y
```

| Parameter | Unit | Description | Validation |
|---|---|---|---|
| `--altitude` | Meters | Altitude above sea level | Finite float |
| `--accuracy` | Meters | Horizontal GPS accuracy radius | Must be >= 0.0 |
| `--speed` | m/s | Ground velocity (meters per second) | Must be >= 0.0 |
| `--heading` | Degrees | Bearing clockwise from True North | 0.0° to 360.0° |

### 6. Location Profiles & Bookmarks
Never re-type coordinates or queries you test repeatedly:
```bash
# Save current location as a bookmark
locsim save office

# Geocode and save a spot in one step
locsim save cafe "Blue Bottle Coffee, San Francisco"

# List all saved bookmarks
locsim list
# or
locsim --bookmarks

# Apply a saved bookmark
locsim use office
# or
locsim -u office

# Delete a bookmark
locsim delete cafe
```

### 7. Mobile Emulator Integration

#### Android ADB
`locsim` automatically identifies online Android Virtual Devices via `adb` and broadcasts fixes:
```bash
# Broadcast to all connected Android emulators
locsim "Times Square, New York" -y

# Target a specific emulator instance
locsim "Times Square, New York" --android-device emulator-5554 -y
```

#### iOS Simulator (macOS)
On macOS, `locsim` synchronizes booted simulators via `xcrun simctl location booted set <lat> <lon>`.

### 8. Pluggable Geocoder Backends
Switch between free and commercial providers on the fly:
```bash
# OpenStreetMap Nominatim (default, no API key needed)
locsim "Brandenburg Gate" --provider nominatim

# Mapbox Places API
locsim "Central Park" --provider mapbox --api-key pk.your_token

# LocationIQ API
locsim "Big Ben" --provider locationiq --api-key your_locationiq_key

# OpenCage Data API
locsim "Colosseum" --provider opencage --api-key your_opencage_key

# Google Maps Geocoding API
locsim "Taj Mahal" --provider google --api-key AIzaSy...
```

### 9. Shell Auto-Completions
Generate high-performance autocompletion scripts:
```bash
# PowerShell (add to your $PROFILE)
locsim completions powershell | Out-String | Invoke-Expression

# Bash (add to ~/.bashrc)
eval "$(locsim completions bash)"

# Zsh (add to ~/.zshrc)
eval "$(locsim completions zsh)"

# Fish
locsim completions fish | source
```

### 10. Diagnostics & Status (`--show` / `-s`)
Inspect the active simulation state across every integration layer:
```bash
locsim --show
```
```text
══════════════════════════════════════
       Simulated Location Status      
══════════════════════════════════════

Status   : ACTIVE SIMULATION
Location : Marwadi University
Address  : Rajkot - Morbi Highway, Rajkot Taluka, Gujarat, India - 360003
Latitude : 22.367601
Longitude: 70.797092
Altitude : 50.00 m
Accuracy : 10.00 m
Speed    : 5.50 m/s
Heading  : 180.00°

OS & Platform Integration:
  Platform   : Windows
  Provider   : Windows Location Provider (WinRT / Sensor Driver Overrides / DevTools)
  Geocoder   : nominatim
  Mechanism  : WinRT Sensor Testing Overrides & Developer State
  Elevated   : No (Standard user)

Artifacts & Exports:
  • ~/.config/locsim/current_location.json [EXISTS]
  • ~/.config/locsim/simulated_location.gpx [EXISTS]
  • ~/.config/locsim/cdp_geolocation.json [EXISTS]
  • ~/.config/locsim/locsim_env.ps1 [EXISTS]

Mobile Emulator Status:
  ADB        : Available (Android Debug Bridge version 1.0.41)
  Emulator   : emulator-5554 [ONLINE]
```

### 11. Reset to Real Physical Location (`reset`)
Clear all simulation overrides, restore defaults, and verify your genuine physical location using IP geolocation:
```bash
locsim reset
```
```text
✓ Simulated location successfully cleared.
  Reset overrides and cleaned up local mock artifacts.

Detecting real physical location via IP Geolocation...

══════════════════════════════════════
       Real Physical Location (IP)    
══════════════════════════════════════

Public IP  : 203.0.113.42
Location   : Rajkot, Gujarat, India
Coordinates: 22.303890, 70.802160
ISP / Org  : Reliance Jio Infocomm Ltd

Device is now reporting its genuine network location.
```

### 12. Interactive Dashboard Menu (`-i` / `--interactive`)
For visual terminal management:
```bash
locsim -i
```
```text
Location Simulator Menu:
1. Search for a location
2. Enter coordinates manually
3. Show current simulated location
4. Clear simulated location
5. Saved location bookmarks (List / Use / Save)
6. Reset to real physical location
7. Exit

Select an option [1-7] > 
```

---

## Configuration & Environment Variables

`locsim` supports configuration files with the following precedence:
1. **CLI Flags** (`--provider`, `--api-key`)
2. **Environment Variables** (`LOCSIM_PROVIDER`, `LOCSIM_API_KEY`)
3. **Project Override** (`./.locsimrc` or `./.locsim.toml`)
4. **Global Defaults** (`~/.config/locsim/config.toml` or `%APPDATA%\locsim\locsim\config\config.toml`)

### `config.toml` Example
```toml
# Default geocoder provider: nominatim | locationiq | mapbox | opencage | google
provider = "nominatim"

# API Key for commercial geocoders (optional for Nominatim)
# api_key = "pk.your_api_key_or_token"

cache_enabled = true
timeout_seconds = 10
user_agent = "locsim/0.1.0 (https://github.com/itznan/locsim; universal-location-simulator)"
```

### Environment Variables
| Variable | Description | Default |
|---|---|---|
| `LOCSIM_PROVIDER` | Geocoder backend (`nominatim`, `locationiq`, `mapbox`, `opencage`, `google`) | `nominatim` |
| `LOCSIM_API_KEY` | API Key applied to the active provider | None |
| `MAPBOX_ACCESS_TOKEN` | Fallback API key for Mapbox Places | None |
| `LOCATIONIQ_API_KEY` | Fallback API key for LocationIQ | None |
| `OPENCAGE_API_KEY` | Fallback API key for OpenCage | None |
| `GOOGLE_MAPS_API_KEY` | Fallback API key for Google Maps | None |
| `LOCSIM_CACHE_ENABLED` | Set to `false` or `0` to bypass local disk cache | `true` |
| `LOCSIM_TIMEOUT` | Request timeout in seconds | `10` |
| `LOCSIM_DATA_DIR` | Custom directory for mock artifacts and caches | OS default |

---

## Automated Testing & CI/CD

### Browser Automation (Playwright / Puppeteer)
`locsim` exports `cdp_geolocation.json` containing exact Chrome DevTools Protocol parameters:

```typescript
// Playwright test example
import { test, expect } from '@playwright/test';
import * as fs from 'fs';

test('verify location-dependent pricing', async ({ browser }) => {
  const cdp = JSON.parse(fs.readFileSync('.locsim/cdp_geolocation.json', 'utf-8'));

  const context = await browser.newContext({
    geolocation: { latitude: cdp.latitude, longitude: cdp.longitude, accuracy: cdp.accuracy },
    permissions: ['geolocation'],
  });

  const page = await context.newPage();
  await page.goto('https://example.com/checkout');
  await expect(page.locator('#currency')).toHaveText('USD');
});
```

### GitHub Actions Workflow Example
```yaml
name: Location-Dependent Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --path .
      - name: Set Simulated Position
        run: |
          locsim "London, UK" -y
          source ~/.config/locsim/locsim_env.sh
          npm test
```

---

## Deep-Dive Documentation

For advanced architecture, developer integrations, and provider guides, explore the dedicated documentation suite:

- [**Architecture & Internals**](docs/ARCHITECTURE.md) — Trait design, memory caching, telemetry validation, and platform dispatcher mechanics.
- [**Geocoding Providers Guide**](docs/PROVIDERS.md) — Comprehensive guide on configuring Nominatim, LocationIQ, Mapbox, OpenCage, and Google Maps.
- [**Mobile Emulator Guide**](docs/EMULATORS.md) — Step-by-step setup for Android Virtual Devices (AVD), multi-device ADB targeting, and iOS Simulators.
- [**Continuous Integration Guide**](docs/CI_INTEGRATION.md) — Playwright, Puppeteer, Selenium, Python `pytest`, and GitHub Actions workflow recipes.
- [**Configuration Reference**](docs/CONFIGURATION.md) — Complete TOML schema, environment variable hierarchy, and export file paths.

---

## Location Simulation Scope & Limitations

When testing location-aware applications, it is essential to understand that different technologies resolve location using different mechanisms:

1. **CLI & Developer Test Suites**: Read `current_location.json` or source `locsim_env.ps1` / `locsim_env.sh`. Fully supported.
2. **OS-Level Location Services**:
   - **Windows**: Updates `HKCU\Software\Locsim`. If run as Administrator, updates the documented Windows Sensor Platform override key (`HKLM\...\Sensor\Overrides\{BFA794E4-F964-4F53-B00F-CEDD57E4457E}`).
   - **Linux**: Generates `geoclue-static.conf` for `org.freedesktop.GeoClue2` and NMEA-0183 (`$GPRMC`, `$GPGGA`) feeds for `gpsd`/`gpsfake`.
   - **macOS**: Executes `xcrun simctl location booted set <lat> <lon>` for running iOS simulators.
3. **Browser Geolocation**: Desktop browsers rely on Wi-Fi positioning services (e.g. Google Location Services). To test browsers, use the exported `cdp_geolocation.json` with Chrome DevTools Sensors or Playwright/Puppeteer.
4. **IP-Based Geolocation (GeoIP)**: Server-side GeoIP lookups (MaxMind, Cloudflare headers) resolve the public IP of your network connection. GPS mock tools cannot alter network routing IP geolocation.

---

## Contributing & Testing

We welcome issues, feedback, and pull requests!

```bash
# Clone the repository
git clone https://github.com/itznan/locsim.git
cd locsim

# Run the full unit and integration test suite
cargo test

# Run linter checks
cargo clippy --all-targets -- -D warnings
```

---

## License

Dual-licensed under either:
- **MIT License** ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

---

<div align="center">
Made by <a href="https://github.com/itznan">itznan</a> and contributors.
</div>


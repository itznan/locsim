# locsim - Universal Location Simulator in Rust

A fast, lightweight, universal CLI-based location simulator written in Rust.

`locsim` enables developers, QA engineers, and testers to resolve any location name or address to geographic coordinates, persist mock locations, and integrate with documented OS-supported testing mechanisms, browser devtools, GPS emulators, and local test suites across **Windows**, **Linux**, and **macOS**.

---

## Features

- **Intuitive Interactive Mode**: Run `locsim` without arguments for an interactive search prompt with clean ASCII banners.
- **Menu-Driven Interactive Mode**: Run `locsim -i` or `locsim --interactive` for a complete management menu (search, manual coordinates, show, clear, exit).
- **Direct Location Resolution**: Search any place name or address directly (e.g. `locsim "Marwadi University, Rajkot"` or `locsim "Mumbai, India"`).
- **Manual Coordinate Input**: Directly set coordinates with rigorous boundary validation (`locsim --lat 22.3072 --lon 73.1812`).
- **Extensible Trait Architecture**: Geocoders and OS location providers are cleanly abstracted behind async and platform traits.
- **Local Response Caching**: Resolves queries via OpenStreetMap Nominatim and caches results locally in JSON for speed and rate-limit compliance.
- **Cross-Platform OS Integration**:
  - **Windows**: Integrates with Windows Registry (`HKCU\Software\Locsim`), Windows Sensor Driver Overrides (`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Sensor\Overrides\{BFA794E4-F964-4F53-B00F-CEDD57E4457E}`), Visual Studio GPX simulation, and PowerShell environment script.
  - **Linux**: Generates `org.freedesktop.GeoClue2` static source configuration (`geoclue-static.conf`), standard NMEA-0183 (`$GPRMC`, `$GPGGA`) feeds for `gpsd`/`gpsfake`, and shell export script.
  - **macOS**: Integrates with Apple Xcode Simulator location simulation (`xcrun simctl location booted set <lat> <lon>`), GPX waypoints, and shell export script.
- **Browser Geolocation DevTools Support**: Automatically exports Chrome DevTools Protocol (`cdp_geolocation.json`) for headless browsers (Playwright, Puppeteer) and Chrome/Edge/Firefox DevTools Sensors.
- **Robust Error Handling**: Powered by `thiserror`, with 100% safe Rust and zero unsafe blocks.

---

## Architecture & Project Structure

```text
src/
├── main.rs            # Application entry point, CLI orchestration, banner, and UI formatters
├── cli.rs             # Command-line arguments definition using clap derive
├── location.rs        # Location model, coordinate boundary validation (-90..90, -180..180)
├── geocoder.rs        # Geocoder trait, Nominatim implementation, and cached layer
├── platform/
│   ├── mod.rs         # LocationProvider trait, diagnostics, GPX, and CDP generators
│   ├── windows.rs     # Windows WinRT / Registry / Sensor Driver provider
│   ├── linux.rs       # Linux GeoClue2 static source / gpsd NMEA provider
│   └── macos.rs       # macOS Xcode simctl / GPX waypoint provider
└── config.rs          # Project directories, configuration, and export paths
```

### Core Traits

#### Geocoder Trait
```rust
#[async_trait]
pub trait Geocoder: Send + Sync {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError>;
    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError>;
}
```

#### LocationProvider Trait
```rust
pub trait LocationProvider: Send + Sync {
    fn set_location(&self, location: &Location) -> Result<(), LocationError>;
    fn get_location(&self) -> Result<Option<Location>, LocationError>;
    fn clear_location(&self) -> Result<(), LocationError>;
    fn provider_name(&self) -> &str;
    fn diagnostics(&self) -> ProviderDiagnostics;
}
```

---

## Installation & Building

### Prerequisites

- [Rust](https://www.rust-lang.org/) (1.75+ or later recommended, Cargo 1.80+)

### Building from Source

Clone the repository and build the release binary:

```bash
git clone https://github.com/locsim/locsim.git
cd locsim

# Build standalone release executable
cargo build --release
```

The resulting standalone executable will be located at:

```text
target/release/locsim        # (Linux / macOS)
target/release/locsim.exe    # (Windows)
```

You can optionally install it into your Cargo bin path:

```bash
cargo install --path .
```

### Running with Administrator Privileges (Windows)

To apply system-wide Windows Sensor Driver Overrides (`HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Sensor\Overrides\...`), run your Command Prompt or PowerShell terminal as **Administrator**:

```cmd
# Run directly from an elevated terminal:
locsim "Mumbai, India"
locsim --show
```

---

## Usage Guide

### 1. Interactive Default Prompt

Simply run `locsim`:

```text
$ locsim

╔══════════════════════════════════════╗
║       Location Simulator             ║
╚══════════════════════════════════════╝

Enter a location:
> Marwadi University, Gauridad, Rajkot, Gujarat, India

Searching location...

✓ Location found

Location : Marwadi University
Address  : Rajkot - Morbi Highway, Rajkot Taluka, Gujarat
           India - 360003

Latitude : 22.367601
Longitude: 70.797092

Set this as simulated location? [Y/n] > y

✓ Simulated location successfully applied!
  Provider: Windows Location Provider (WinRT / Sensor Driver Overrides / DevTools)
  OS Mechanism: WinRT Sensor Testing Overrides & Developer State

Generated integration files:
  • C:\Users\user\AppData\Roaming\locsim\locsim\config\current_location.json
  • C:\Users\user\AppData\Roaming\locsim\locsim\config\simulated_location.gpx
  • C:\Users\user\AppData\Roaming\locsim\locsim\config\cdp_geolocation.json
  • C:\Users\user\AppData\Roaming\locsim\locsim\config\locsim_env.ps1
```

### 2. Direct Search by Location Name

```bash
# Search and prompt confirmation
locsim "Mumbai, India"

# Search and automatically accept confirmation (-y / --yes)
locsim "Marwadi University, Rajkot" -y
```

### 3. Manual Coordinate Input

Coordinates are validated to ensure `-90.0 <= lat <= 90.0` and `-180.0 <= lon <= 180.0`:

```bash
locsim --lat 22.3072 --lon 73.1812
```

### 4. Display Current Simulated Location & Diagnostics

```bash
locsim --show
# or
locsim -s
```

Outputs the active location, status of exported artifacts, and an architectural breakdown distinguishing OS, browser, and network layers.

### 5. Clear Simulated Location

```bash
locsim --clear
# or
locsim -c
```

Removes simulated location from the OS provider and deletes generated mock files.

### 6. Interactive Menu Mode

```bash
locsim --interactive
# or
locsim -i
```

Presents a menu:

```text
Location Simulator Menu:
1. Search for a location
2. Enter coordinates manually
3. Show current simulated location
4. Clear simulated location
5. Exit

Select an option [1-5] > 
```

---

## Configuration & Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LOCSIM_GEOCODER_URL` | Forward geocoding API URL | `https://nominatim.openstreetmap.org/search` |
| `LOCSIM_REVERSE_GEOCODER_URL` | Reverse geocoding API URL | `https://nominatim.openstreetmap.org/reverse` |
| `LOCSIM_CACHE_ENABLED` | Enable/disable local query cache (`true`/`false`) | `true` |
| `LOCSIM_TIMEOUT` | Network request timeout in seconds | `10` |
| `LOCSIM_DATA_DIR` | Custom directory path for configs and exports | System application directory |

---

## Location Simulation Scope & Limitations

When testing location-aware applications, it is essential to understand that different technologies resolve location using different mechanisms. Changing simulated coordinates in one layer does not automatically alter all layers:

1. **CLI & Developer Test Suites**:
   - `locsim` writes `current_location.json` and a shell environment script (`locsim_env.ps1` on Windows, `locsim_env.sh` on Unix).
   - Test suites, CLI utilities, and developer scripts can read these directly to test location-dependent behavior.

2. **OS-Level Location Services**:
   - **Windows**: `locsim` updates `HKCU\Software\Locsim`. If run as Administrator, it updates the documented Windows Sensor Platform override key (`HKLM\...\Sensor\Overrides\{BFA794E4-F964-4F53-B00F-CEDD57E4457E}`) for sensor and driver testing harnesses.
   - **Linux**: `locsim` generates `geoclue-static.conf` for `org.freedesktop.GeoClue2` and generates NMEA-0183 (`$GPRMC`, `$GPGGA`) sentences in `gps_nmea.txt` for `gpsd`/`gpsfake`.
   - **macOS**: `locsim` executes `xcrun simctl location booted set <lat> <lon>` for running iOS/watchOS simulators and produces GPX waypoint tracks for Xcode Instruments.

3. **Browser Geolocation**:
   - Desktop browsers (Chrome, Edge, Firefox, Brave) typically query Wi-Fi positioning services (e.g. Google Location Services) rather than hardware GPS.
   - To mock geolocation in browsers, `locsim` exports `cdp_geolocation.json`. You can use this with Chrome DevTools (`More tools > Sensors > Location`) or automated frameworks (Playwright, Puppeteer) via `Emulation.setGeolocationOverride`.

4. **IP-Based Geolocation (GeoIP)**:
   - Websites or APIs that determine location on the server based on the incoming TCP/IP connection (e.g., MaxMind GeoIP, Cloudflare headers) see your ISP or VPN's public IP address. GPS mock tools cannot alter network routing IP geolocation.

5. **Independent Native Applications**:
   - Applications that directly access hardware GPS dongles via proprietary drivers or scan local Wi-Fi BSSIDs independently bypass simulated OS coordinates unless custom virtual sensor drivers are loaded.

---

## Running Unit Tests

Run the test suite with:

```bash
cargo test
```

All tests cover:
- Coordinate boundary validation (-90..90, -180..180, NaN, and Infinity checks)
- JSON serialization and deserialization
- Geocoder caching and query normalization
- Platform GPX XML generation and XML entity escaping
- Chrome DevTools Protocol (CDP) payload format
- Linux NMEA 0183 checksum and sentence formatting
- GeoClue configuration generation

---

## License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

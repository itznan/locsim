# Architecture & Internals of `locsim`

`locsim` is designed around a decoupled, modular architecture in 100% safe Rust. Its primary objective is to act as a **single source of truth for location simulation**, propagating simulated coordinates and telemetry to all testing layers (CLI, OS services, mobile emulators, browser test harnesses, and GPS feeds).

---

## 1. System Overview

```text
                               ┌──────────────────────────┐
                               │   CLI / User / CI Script │
                               └────────────┬─────────────┘
                                            │
                                            ▼
                               ┌──────────────────────────┐
                               │      CLI Parser (Clap)   │
                               │  flags, options, subcmds │
                               └────────────┬─────────────┘
                                            │
                     ┌──────────────────────┴──────────────────────┐
                     │                                             │
                     ▼                                             ▼
          ┌─────────────────────┐                       ┌─────────────────────┐
          │   Config Manager    │                       │  Bookmark Manager   │
          │ TOML / Env / Flags  │                       │   bookmarks.json    │
          └──────────┬──────────┘                       └──────────┬──────────┘
                     │                                             │
                     ▼                                             │
          ┌─────────────────────┐                                  │
          │   Cached Geocoder   │                                  │
          │  RwLock Memory + FS │                                  │
          └──────────┬──────────┘                                  │
                     │                                             │
                     ▼                                             │
          ┌────────────────────────────────────────────────────────┴───┐
          │                       Location Core                        │
          │  - Latitude & Longitude validation (-90..90, -180..180)    │
          │  - Telemetry (Altitude, Accuracy, Speed, Heading)          │
          │  - Timestamp & Formatted Address                           │
          └──────────────────────────────┬─────────────────────────────┘
                                         │
                                         ▼
                     ┌───────────────────────────────────────┐
                     │       Platform Dispatcher Engine      │
                     └───────────────────┬───────────────────┘
                                         │
     ┌──────────────────┬────────────────┼─────────────────┬──────────────────┐
     ▼                  ▼                ▼                 ▼                  ▼
┌───────────┐    ┌─────────────┐   ┌────────────┐    ┌───────────┐    ┌───────────────┐
│  Windows  │    │    Linux    │   │   macOS    │    │  Android  │    │  Export Files │
│  WinRT &  │    │  GeoClue2 & │   │   Xcode    │    │    ADB    │    │  CDP, GPX,    │
│  Sensors  │    │  gpsd NMEA  │   │   simctl   │    │  emu fix  │    │  JSON, Shell  │
└───────────┘    └─────────────┘   └────────────┘    └───────────┘    └───────────────┘
```

---

## 2. Core Traits & Abstractions

### A. `Geocoder` Trait ([`src/geocoder.rs`](file:///E:/NAN/Github/locor/src/geocoder.rs))

All geocoding services implement the asynchronous `Geocoder` trait:

```rust
#[async_trait]
pub trait Geocoder: Send + Sync {
    /// Forward geocode a human-readable query or address into a validated Location.
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError>;

    /// Reverse geocode coordinates (lat, lon) into a human-readable Location.
    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError>;
}
```

#### Trait Implementations:
- **`NominatimGeocoder`**: Queries OpenStreetMap Nominatim with rate-respectful user agents and timeouts.
- **`LocationIqGeocoder`**: Queries LocationIQ API with dedicated API keys and higher rate limits.
- **`MapboxGeocoder`**: Queries Mapbox Geocoding v5 Places API using access tokens.
- **`OpenCageGeocoder`**: Queries OpenCage Data forward & reverse endpoints.
- **`GoogleMapsGeocoder`**: Queries Google Maps Geocoding API (`maps.googleapis.com`).
- **`Box<dyn Geocoder>`**: Allows dynamic dispatch across backends selected at runtime.
- **`CachedGeocoder<G>`**: A generic transparent decorator that wraps any `Geocoder`, maintaining an in-memory `RwLock<HashMap<String, Location>>` synchronized with `geocode_cache.json` on disk.

### B. `LocationProvider` Trait ([`src/platform/mod.rs`](file:///E:/NAN/Github/locor/src/platform/mod.rs))

Operating system integrations implement the `LocationProvider` trait:

```rust
pub trait LocationProvider: Send + Sync {
    /// Apply and persist simulated location to OS and developer mock files.
    fn set_location(&self, location: &Location) -> Result<(), LocationError>;

    /// Retrieve the currently active simulated location, if any.
    fn get_location(&self) -> Result<Option<Location>, LocationError>;

    /// Clear all active simulated locations and clean up mock artifacts.
    fn clear_location(&self) -> Result<(), LocationError>;

    /// Return a human-readable name of the platform provider.
    fn provider_name(&self) -> &str;

    /// Return diagnostic metadata describing elevation, exported files, and OS mechanisms.
    fn diagnostics(&self) -> ProviderDiagnostics;
}
```

---

## 3. Telemetry & Data Model ([`src/location.rs`](file:///E:/NAN/Github/locor/src/location.rs))

The `Location` struct holds the complete state of a geographic position:

```rust
pub struct Location {
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,   // Meters above WGS84 ellipsoid
    pub accuracy: Option<f64>,   // Horizontal accuracy radius in meters
    pub speed: Option<f64>,      // Ground speed in meters/second
    pub heading: Option<f64>,    // Degrees (0.0° - 360.0° clockwise from true north)
    pub timestamp: String,       // RFC3339 UTC timestamp
}
```

### Validation Invariants
- **Latitude**: Must be finite and within `[-90.0, 90.0]`.
- **Longitude**: Must be finite and within `[-180.0, 180.0]`.
- **Accuracy**: Must be non-negative and finite (`accuracy >= 0.0`).
- **Speed**: Must be non-negative and finite (`speed >= 0.0`).
- **Heading**: Must be within `[0.0, 360.0]`.
- **Altitude**: Must be finite (`f64::is_finite`).

---

## 4. Platform Dispatch Mechanics

### Windows ([`src/platform/windows.rs`](file:///E:/NAN/Github/locor/src/platform/windows.rs))
1. **Registry User State**: Writes `HKCU\Software\Locsim` values (`Latitude`, `Longitude`, `Name`, `Address`, `Altitude`, `Accuracy`, `Speed`, `Heading`).
2. **Sensor Driver Overrides**: If executed from an elevated prompt (Administrator), updates `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Sensor\Overrides\{BFA794E4-F964-4F53-B00F-CEDD57E4457E}` for Windows Location Platform testing harnesses.
3. **PowerShell Environment**: Writes `locsim_env.ps1` with environment exports.

### Linux ([`src/platform/linux.rs`](file:///E:/NAN/Github/locor/src/platform/linux.rs))
1. **GeoClue2**: Generates `geoclue-static.conf` with a `[static-source]` section for `org.freedesktop.GeoClue2`.
2. **NMEA 0183 Stream**: Formats `$GPRMC` (speed knots, bearing, coordinates) and `$GPGGA` (altitude, satellites, fix quality) with XOR checksums for `gpsd` and `gpsfake`.
3. **Bash Environment**: Writes `locsim_env.sh` with export statements.

### macOS ([`src/platform/macos.rs`](file:///E:/NAN/Github/locor/src/platform/macos.rs))
1. **iOS Simulator**: Executes `xcrun simctl location booted set <lat> <lon>` to automatically set location on any running iOS/iPadOS/watchOS simulator.
2. **GPX Waypoints**: Generates standard GPX 1.1 XML for Xcode Instruments and Apple Developer tools.

### Android Emulator ([`src/platform/android.rs`](file:///E:/NAN/Github/locor/src/platform/android.rs))
1. Locates `adb` binary dynamically.
2. Checks connected emulators via `adb devices`.
3. Dispatches `adb -s <serial> emu geo fix <lon> <lat> [alt]`.
4. Falls back to `adb -s <serial> shell cmd location set-location <lat> <lon>`.

### Universal Browser CDP ([`src/platform/mod.rs`](file:///E:/NAN/Github/locor/src/platform/mod.rs))
Generates `cdp_geolocation.json`:
```json
{
  "latitude": 37.7749,
  "longitude": -122.4194,
  "accuracy": 10.0,
  "altitude": 15.0,
  "altitudeAccuracy": 5.0,
  "heading": 180.0,
  "speed": 4.5
}
```
Directly consumable by Playwright, Puppeteer, Selenium, or Chrome DevTools Sensors.

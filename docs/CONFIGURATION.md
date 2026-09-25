# Configuration Reference for `locsim`

`locsim` allows fine-grained configuration through configuration files, environment variables, and CLI parameters.

---

## 1. Resolution Precedence

When resolving settings, `locsim` uses the following order of precedence (highest to lowest):

1. **CLI Arguments & Flags** (e.g. `--provider mapbox --api-key pk...`)
2. **Environment Variables** (e.g. `LOCSIM_PROVIDER`, `LOCSIM_API_KEY`)
3. **Project-Local Configuration File** (`./.locsimrc` or `./.locsim.toml` in the current working directory)
4. **Global Configuration File** (`~/.config/locsim/config.toml` or `%APPDATA%\locsim\locsim\config\config.toml`)
5. **Built-in Defaults**

---

## 2. Configuration File Schema (TOML)

Create a `config.toml` or `.locsimrc` file:

```toml
# ============================================================
# locsim Configuration File
# ============================================================

# Default Geocoder Provider:
# Options: "nominatim" (default), "locationiq", "mapbox", "opencage", "google"
provider = "nominatim"

# API Key or Access Token (required for commercial providers)
# api_key = "pk.your_api_key_or_token"

# Enable or disable local query cache (~/.config/locsim/geocode_cache.json)
cache_enabled = true

# Network HTTP timeout in seconds
timeout_seconds = 10

# HTTP User-Agent sent with outgoing geocoding requests
user_agent = "locsim/0.1.0 (https://github.com/itznan/locsim; universal-location-simulator)"

# Custom Geocoding API URLs (overrides provider defaults)
# geocoder_url = "https://nominatim.openstreetmap.org/search"
# reverse_geocoder_url = "https://nominatim.openstreetmap.org/reverse"
```

---

## 3. Environment Variables Reference

| Environment Variable | Description | Default |
|---|---|---|
| `LOCSIM_PROVIDER` | Active geocoder provider (`nominatim`, `locationiq`, `mapbox`, `opencage`, `google`) | `nominatim` |
| `LOCSIM_API_KEY` | General API key applied to the active provider | None |
| `MAPBOX_ACCESS_TOKEN` | Fallback API key specifically for Mapbox Places | None |
| `LOCATIONIQ_API_KEY` | Fallback API key specifically for LocationIQ | None |
| `OPENCAGE_API_KEY` | Fallback API key specifically for OpenCage | None |
| `GOOGLE_MAPS_API_KEY` | Fallback API key specifically for Google Maps | None |
| `LOCSIM_GEOCODER_URL` | Custom endpoint for forward geocoding queries | Provider default |
| `LOCSIM_REVERSE_GEOCODER_URL` | Custom endpoint for reverse geocoding queries | Provider default |
| `LOCSIM_CACHE_ENABLED` | Set to `0` or `false` to disable caching | `true` |
| `LOCSIM_TIMEOUT` | Request timeout in seconds | `10` |
| `LOCSIM_DATA_DIR` | Custom directory path for all configuration, cache, and exported artifacts | OS application directory |

---

## 4. Export File Locations

All generated mock files and cached locations are stored in the `locsim` data directory:

| OS | Default Directory Path |
|---|---|
| **Windows** | `%APPDATA%\locsim\locsim\config\` (e.g. `C:\Users\<user>\AppData\Roaming\locsim\locsim\config\`) |
| **Linux** | `~/.config/locsim/` |
| **macOS** | `~/Library/Application Support/com.locsim.locsim/` |

### Managed Files:
- `current_location.json`: Active location state (coordinates, address, telemetry, timestamp).
- `bookmarks.json`: Saved location bookmarks and profiles.
- `geocode_cache.json`: Cache of normalized forward/reverse geocoding responses.
- `simulated_location.gpx`: Standard GPX 1.1 waypoint track for Xcode, Visual Studio, and GPS devices.
- `cdp_geolocation.json`: Chrome DevTools Protocol geolocation override for Chromium-based automated testing.
- `locsim_env.ps1` (Windows): PowerShell script setting environment variables for the active simulated position.
- `locsim_env.sh` (Unix): Bash script setting environment variables for the active simulated position.
- `geoclue-static.conf` (Linux): Configuration snippet for `org.freedesktop.GeoClue2`.
- `gps_nmea.txt` (Linux): NMEA 0183 (`$GPRMC`, `$GPGGA`) GPS data feed for `gpsd`/`gpsfake`.

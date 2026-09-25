# Geocoding Providers in `locsim`

`locsim` supports 5 distinct geocoding backends. You can switch providers on the fly using `--provider <NAME>` or configure your favorite default in `config.toml` or environment variables.

---

## Provider Comparison

| Provider | Name | API Key Required? | Forward Geocoding | Reverse Geocoding | Best For |
|---|---|---|:---:|:---:|---|
| **Nominatim** | `nominatim` (or `osm`) | ❌ No | ✅ Yes | ✅ Yes | Out-of-the-box local developer testing without signing up for services |
| **LocationIQ** | `locationiq` (or `iq`) | ✅ Yes | ✅ Yes | ✅ Yes | High-volume OpenStreetMap queries with generous free tiers |
| **Mapbox** | `mapbox` | ✅ Yes | ✅ Yes | ✅ Yes | High-accuracy commercial POI data and mobile navigation apps |
| **OpenCage** | `opencage` (or `cage`) | ✅ Yes | ✅ Yes | ✅ Yes | Global addressing, forward/reverse worldwide reverse geocoding |
| **Google Maps** | `google` | ✅ Yes | ✅ Yes | ✅ Yes | Enterprise apps and Google Places address formats |

---

## 1. OpenStreetMap Nominatim (`nominatim`)

Nominatim is the default provider in `locsim`. It requires no API key or account creation.

### How to use:
```bash
locsim "Brandenburg Gate, Berlin"
locsim --provider nominatim "Colosseum, Rome"
```

### Rate Limits & Policy:
- Nominatim has a strict community usage policy: maximum 1 request per second.
- `locsim` automatically respects this by sending a descriptive User-Agent (`locsim/0.1.0 (https://github.com/itznan/locsim)`) and caching all resolved locations locally in `geocode_cache.json`.

---

## 2. LocationIQ (`locationiq`)

LocationIQ provides commercial-grade OpenStreetMap search with high availability and fast response times.

### Obtaining an API key:
1. Sign up at [LocationIQ](https://locationiq.com/).
2. Grab your access token from the dashboard.

### How to use:
```bash
# Via CLI flag:
locsim "Sydney Harbour Bridge" --provider locationiq --api-key YOUR_LOCATIONIQ_TOKEN

# Via Environment Variable:
export LOCATIONIQ_API_KEY="YOUR_LOCATIONIQ_TOKEN"
locsim "Sydney Harbour Bridge" --provider locationiq
```

---

## 3. Mapbox Places (`mapbox`)

Mapbox Geocoding v5 offers pinpoint accuracy for street addresses, points of interest, and landmarks worldwide.

### Obtaining an Access Token:
1. Sign up at [Mapbox](https://account.mapbox.com/).
2. Create a Default Public Token (`pk....`).

### How to use:
```bash
# Via CLI flag:
locsim "Times Square, New York" --provider mapbox --api-key pk.eyJ1...

# Via Environment Variable:
export MAPBOX_ACCESS_TOKEN="pk.eyJ1..."
locsim "Times Square, New York" --provider mapbox
```

---

## 4. OpenCage Data (`opencage`)

OpenCage aggregates OpenStreetMap, twofishes, and open address databases into a unified forward and reverse geocoding API.

### Obtaining an API key:
1. Sign up at [OpenCage](https://opencagedata.com/).
2. Copy your 32-character API key.

### How to use:
```bash
# Forward search:
locsim "Arc de Triomphe, Paris" --provider opencage --api-key YOUR_OPENCAGE_KEY

# Reverse search:
locsim -r 48.8738 2.2950 --provider opencage --api-key YOUR_OPENCAGE_KEY
```

---

## 5. Google Maps Geocoding API (`google`)

Direct integration with Google Maps Geocoding API (`https://maps.googleapis.com/maps/api/geocode/json`).

### Obtaining an API key:
1. Create a project in [Google Cloud Console](https://console.cloud.google.com/).
2. Enable the **Geocoding API**.
3. Generate an API Key under Credentials.

### How to use:
```bash
# Via CLI flag:
locsim "Burj Khalifa, Dubai" --provider google --api-key AIzaSy...

# Via Environment Variable:
export GOOGLE_MAPS_API_KEY="AIzaSy..."
locsim "Burj Khalifa, Dubai" --provider google
```

---

## Setting a Permanent Provider in `config.toml`

To avoid typing `--provider` and `--api-key` every time, save them in your configuration file:

**`~/.config/locsim/config.toml`** (Linux/macOS)  
**`%APPDATA%\locsim\locsim\config\config.toml`** (Windows)

```toml
provider = "mapbox"
api_key = "pk.eyJ1..."
cache_enabled = true
timeout_seconds = 15
```

Now any `locsim` command will automatically use your configured provider and token.

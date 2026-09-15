use crate::config::{AppPaths, Config};
use crate::location::{Location, LocationValidationError};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::RwLock;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum GeocodeError {
    #[error("Location not found for query: '{0}'")]
    NotFound(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Invalid coordinate returned: {0}")]
    InvalidCoordinate(#[from] LocationValidationError),

    #[error("Rate limit exceeded by geocoding provider (HTTP 429). Please wait a moment.")]
    RateLimited,

    #[error("Failed to parse geocoding response: {0}")]
    ParseError(String),

    #[error("I/O error with cache: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Geocoding service error: {0}")]
    Service(String),
}

/// Abstract trait for geocoding providers.
#[async_trait]
pub trait Geocoder: Send + Sync {
    /// Forward geocode a place name or address into a Location with coordinates.
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError>;

    /// Reverse geocode coordinates into a human-readable Location.
    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError>;
}

/// Nominatim API search response item
#[derive(Debug, Deserialize)]
struct NominatimItem {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub lat: String,
    pub lon: String,
    pub address: Option<NominatimAddress>,
}

#[derive(Debug, Deserialize)]
struct NominatimAddress {
    pub road: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub town: Option<String>,
    pub village: Option<String>,
    pub county: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub postcode: Option<String>,
}

/// OpenStreetMap Nominatim Geocoder implementation with rate-respectful headers.
pub struct NominatimGeocoder {
    client: reqwest::Client,
    config: Config,
}

impl NominatimGeocoder {
    pub fn new(config: Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_default();

        Self { client, config }
    }
}

#[async_trait]
impl Geocoder for NominatimGeocoder {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(GeocodeError::NotFound("Empty query provided".to_string()));
        }

        let resp = self
            .client
            .get(&self.config.geocoder_url)
            .query(&[
                ("q", trimmed),
                ("format", "jsonv2"),
                ("addressdetails", "1"),
                ("limit", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "HTTP status: {}",
                resp.status()
            )));
        }

        let items: Vec<NominatimItem> = resp.json().await?;
        let first = items
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(trimmed.to_string()))?;

        let lat: f64 = first
            .lat
            .parse()
            .map_err(|_| GeocodeError::ParseError(format!("Invalid latitude string: {}", first.lat)))?;
        let lon: f64 = first
            .lon
            .parse()
            .map_err(|_| GeocodeError::ParseError(format!("Invalid longitude string: {}", first.lon)))?;

        let display_name = first.display_name.unwrap_or_else(|| trimmed.to_string());
        let name = first.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| {
            // Pick first segment of display_name as title
            display_name
                .split(',')
                .next()
                .unwrap_or(trimmed)
                .trim()
                .to_string()
        });

        // Format nice address
        let address = if let Some(addr) = first.address {
            let mut parts = Vec::new();
            if let Some(r) = addr.road {
                parts.push(r);
            }
            if let Some(s) = addr.suburb {
                parts.push(s);
            }
            let city_part = addr.city.or(addr.town).or(addr.village).or(addr.county);
            if let Some(c) = city_part {
                parts.push(c);
            }
            if let Some(st) = addr.state {
                parts.push(st);
            }
            if let Some(country) = addr.country {
                if let Some(p) = addr.postcode {
                    parts.push(format!("{} - {}", country, p));
                } else {
                    parts.push(country);
                }
            }
            if parts.is_empty() {
                display_name
            } else {
                parts.join(", ")
            }
        } else {
            display_name
        };

        Location::new(name, address, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        Location::validate_coordinates(lat, lon).map_err(GeocodeError::InvalidCoordinate)?;

        let lat_str = lat.to_string();
        let lon_str = lon.to_string();
        let resp = self
            .client
            .get(&self.config.reverse_geocoder_url)
            .query(&[
                ("lat", lat_str.as_str()),
                ("lon", lon_str.as_str()),
                ("format", "jsonv2"),
                ("addressdetails", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "Reverse geocode HTTP status: {}",
                resp.status()
            )));
        }

        let item: NominatimItem = resp.json().await?;
        let display = item
            .display_name
            .unwrap_or_else(|| format!("{:.5}, {:.5}", lat, lon));
        let name = item
            .name
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| display.split(',').next().unwrap_or("Custom Location").trim().to_string());

        Location::new(name, display, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }
}

/// Caching wrapper around any Geocoder implementation.
pub struct CachedGeocoder<G: Geocoder> {
    inner: G,
    cache: RwLock<HashMap<String, Location>>,
    cache_file: std::path::PathBuf,
    enabled: bool,
}

impl<G: Geocoder> CachedGeocoder<G> {
    pub fn new(inner: G, config: &Config) -> Self {
        let cache_file = AppPaths::cache_file();
        let mut cache_map = HashMap::new();

        if config.cache_enabled && cache_file.exists() {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok(loaded) = serde_json::from_str::<HashMap<String, Location>>(&content) {
                    cache_map = loaded;
                }
            }
        }

        Self {
            inner,
            cache: RwLock::new(cache_map),
            cache_file,
            enabled: config.cache_enabled,
        }
    }

    fn normalize_key(query: &str) -> String {
        query.trim().to_lowercase()
    }

    fn persist_cache(&self) {
        if !self.enabled {
            return;
        }

        if let Ok(guard) = self.cache.read() {
            if let Ok(json) = serde_json::to_string_pretty(&*guard) {
                let _ = fs::write(&self.cache_file, json);
            }
        }
    }
}

#[async_trait]
impl<G: Geocoder> Geocoder for CachedGeocoder<G> {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        let key = Self::normalize_key(query);

        if self.enabled {
            if let Ok(guard) = self.cache.read() {
                if let Some(loc) = guard.get(&key) {
                    return Ok(loc.clone());
                }
            }
        }

        let loc = self.inner.geocode(query).await?;

        if self.enabled {
            if let Ok(mut guard) = self.cache.write() {
                guard.insert(key, loc.clone());
            }
            self.persist_cache();
        }

        Ok(loc)
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        let key = format!("{:.4},{:.4}", lat, lon);

        if self.enabled {
            if let Ok(guard) = self.cache.read() {
                if let Some(loc) = guard.get(&key) {
                    return Ok(loc.clone());
                }
            }
        }

        let loc = match self.inner.reverse_geocode(lat, lon).await {
            Ok(l) => l,
            Err(_) => {
                // Gracefully generate coordinate-based Location if reverse geocoding is unavailable
                Location::new(
                    format!("{:.5}, {:.5}", lat, lon),
                    format!("Manual Coordinates (Lat: {:.5}, Lon: {:.5})", lat, lon),
                    lat,
                    lon,
                )?
            }
        };

        if self.enabled {
            if let Ok(mut guard) = self.cache.write() {
                guard.insert(key, loc.clone());
            }
            self.persist_cache();
        }

        Ok(loc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockGeocoder {
        locations: HashMap<String, Location>,
    }

    #[async_trait]
    impl Geocoder for MockGeocoder {
        async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
            self.locations
                .get(query)
                .cloned()
                .ok_or_else(|| GeocodeError::NotFound(query.to_string()))
        }

        async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
            Location::new("Mock Location", "Mock Address", lat, lon)
                .map_err(GeocodeError::InvalidCoordinate)
        }
    }

    #[tokio::test]
    async fn test_cached_geocoder_hit() {
        let mut mock_data = HashMap::new();
        let rajkot = Location::new("Marwadi University", "Gauridad, Rajkot", 22.3688, 70.8022).unwrap();
        mock_data.insert("Marwadi University".to_string(), rajkot.clone());

        let mock = MockGeocoder { locations: mock_data };
        let mut config = Config::default();
        config.cache_enabled = true;

        let cached = CachedGeocoder::new(mock, &config);

        // First call should resolve
        let res = cached.geocode("Marwadi University").await.unwrap();
        assert_eq!(res.name, "Marwadi University");
        assert_eq!(res.latitude, 22.3688);

        // Check internal cache has key
        let key = CachedGeocoder::<MockGeocoder>::normalize_key("Marwadi University");
        let guard = cached.cache.read().unwrap();
        assert!(guard.contains_key(&key));
    }

    #[tokio::test]
    async fn test_geocoder_not_found() {
        let mock = MockGeocoder {
            locations: HashMap::new(),
        };
        let config = Config::default();
        let cached = CachedGeocoder::new(mock, &config);

        let err = cached.geocode("Unknown Place That Does Not Exist").await;
        assert!(matches!(err, Err(GeocodeError::NotFound(_))));
    }
}

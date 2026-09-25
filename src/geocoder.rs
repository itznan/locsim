use crate::config::{AppPaths, Config, GeocoderProvider};
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

    #[error("API key required for provider '{0}'. Pass --api-key <KEY> or set environment variable.")]
    MissingApiKey(String),

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

/// LocationIQ Geocoder (Nominatim-compatible with API key)
pub struct LocationIqGeocoder {
    client: reqwest::Client,
    api_key: String,
}

impl LocationIqGeocoder {
    pub fn new(api_key: String, config: &Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_default();

        Self { client, api_key }
    }
}

#[async_trait]
impl Geocoder for LocationIqGeocoder {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(GeocodeError::NotFound("Empty query provided".to_string()));
        }

        let resp = self
            .client
            .get("https://us1.locationiq.com/v1/search")
            .query(&[
                ("key", self.api_key.as_str()),
                ("q", trimmed),
                ("format", "json"),
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
                "LocationIQ HTTP status: {}",
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
            .map_err(|_| GeocodeError::ParseError(format!("Invalid latitude: {}", first.lat)))?;
        let lon: f64 = first
            .lon
            .parse()
            .map_err(|_| GeocodeError::ParseError(format!("Invalid longitude: {}", first.lon)))?;

        let display_name = first.display_name.unwrap_or_else(|| trimmed.to_string());
        let name = first.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| {
            display_name
                .split(',')
                .next()
                .unwrap_or(trimmed)
                .trim()
                .to_string()
        });

        Location::new(name, display_name, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        Location::validate_coordinates(lat, lon).map_err(GeocodeError::InvalidCoordinate)?;

        let lat_str = lat.to_string();
        let lon_str = lon.to_string();
        let resp = self
            .client
            .get("https://us1.locationiq.com/v1/reverse")
            .query(&[
                ("key", self.api_key.as_str()),
                ("lat", lat_str.as_str()),
                ("lon", lon_str.as_str()),
                ("format", "json"),
                ("addressdetails", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "LocationIQ reverse HTTP status: {}",
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

/// Mapbox Geocoding API response
#[derive(Debug, Deserialize)]
struct MapboxResponse {
    pub features: Vec<MapboxFeature>,
}

#[derive(Debug, Deserialize)]
struct MapboxFeature {
    pub text: Option<String>,
    pub place_name: Option<String>,
    pub center: Option<Vec<f64>>, // [lon, lat]
}

/// Mapbox Places Geocoder
pub struct MapboxGeocoder {
    client: reqwest::Client,
    access_token: String,
}

impl MapboxGeocoder {
    pub fn new(access_token: String, config: &Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_default();

        Self {
            client,
            access_token,
        }
    }
}

#[async_trait]
impl Geocoder for MapboxGeocoder {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(GeocodeError::NotFound("Empty query provided".to_string()));
        }

        let mut url = reqwest::Url::parse("https://api.mapbox.com/geocoding/v5/mapbox.places/").unwrap();
        if let Ok(mut segments) = url.path_segments_mut() {
            segments.push(&format!("{}.json", trimmed));
        }

        let resp = self
            .client
            .get(url)
            .query(&[
                ("access_token", self.access_token.as_str()),
                ("limit", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "Mapbox HTTP status: {}",
                resp.status()
            )));
        }

        let data: MapboxResponse = resp.json().await?;
        let first = data
            .features
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(trimmed.to_string()))?;

        let center = first
            .center
            .filter(|c| c.len() >= 2)
            .ok_or_else(|| GeocodeError::ParseError("Missing center coordinate from Mapbox".to_string()))?;

        let lon = center[0];
        let lat = center[1];
        let name = first.text.unwrap_or_else(|| trimmed.to_string());
        let address = first.place_name.unwrap_or_else(|| name.clone());

        Location::new(name, address, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        Location::validate_coordinates(lat, lon).map_err(GeocodeError::InvalidCoordinate)?;

        let mut url = reqwest::Url::parse("https://api.mapbox.com/geocoding/v5/mapbox.places/").unwrap();
        if let Ok(mut segments) = url.path_segments_mut() {
            segments.push(&format!("{:.6},{:.6}.json", lon, lat));
        }

        let resp = self
            .client
            .get(url)
            .query(&[
                ("access_token", self.access_token.as_str()),
                ("limit", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "Mapbox reverse HTTP status: {}",
                resp.status()
            )));
        }

        let data: MapboxResponse = resp.json().await?;
        let first = data
            .features
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(format!("{:.6}, {:.6}", lat, lon)))?;

        let name = first
            .text
            .unwrap_or_else(|| format!("{:.5}, {:.5}", lat, lon));
        let address = first.place_name.unwrap_or_else(|| name.clone());

        Location::new(name, address, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }
}

/// OpenCage Data Geocoder response
#[derive(Debug, Deserialize)]
struct OpenCageResponse {
    pub results: Vec<OpenCageResult>,
}

#[derive(Debug, Deserialize)]
struct OpenCageResult {
    pub formatted: Option<String>,
    pub geometry: Option<OpenCageGeometry>,
}

#[derive(Debug, Deserialize)]
struct OpenCageGeometry {
    pub lat: f64,
    pub lng: f64,
}

/// OpenCage Data Geocoder
pub struct OpenCageGeocoder {
    client: reqwest::Client,
    api_key: String,
}

impl OpenCageGeocoder {
    pub fn new(api_key: String, config: &Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_default();

        Self { client, api_key }
    }
}

#[async_trait]
impl Geocoder for OpenCageGeocoder {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(GeocodeError::NotFound("Empty query provided".to_string()));
        }

        let resp = self
            .client
            .get("https://api.opencagedata.com/geocode/v1/json")
            .query(&[
                ("q", trimmed),
                ("key", self.api_key.as_str()),
                ("limit", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "OpenCage HTTP status: {}",
                resp.status()
            )));
        }

        let data: OpenCageResponse = resp.json().await?;
        let first = data
            .results
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(trimmed.to_string()))?;

        let geom = first
            .geometry
            .ok_or_else(|| GeocodeError::ParseError("Missing geometry in OpenCage response".to_string()))?;

        let address = first.formatted.unwrap_or_else(|| trimmed.to_string());
        let name = address
            .split(',')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .to_string();

        Location::new(name, address, geom.lat, geom.lng).map_err(GeocodeError::InvalidCoordinate)
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        Location::validate_coordinates(lat, lon).map_err(GeocodeError::InvalidCoordinate)?;

        let q = format!("{:.6}+{:.6}", lat, lon);
        let resp = self
            .client
            .get("https://api.opencagedata.com/geocode/v1/json")
            .query(&[
                ("q", q.as_str()),
                ("key", self.api_key.as_str()),
                ("limit", "1"),
            ])
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(GeocodeError::RateLimited);
        }

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "OpenCage reverse HTTP status: {}",
                resp.status()
            )));
        }

        let data: OpenCageResponse = resp.json().await?;
        let first = data
            .results
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(format!("{:.6}, {:.6}", lat, lon)))?;

        let address = first
            .formatted
            .unwrap_or_else(|| format!("{:.5}, {:.5}", lat, lon));
        let name = address
            .split(',')
            .next()
            .unwrap_or("Custom Location")
            .trim()
            .to_string();

        Location::new(name, address, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }
}

/// Google Maps Geocoder response
#[derive(Debug, Deserialize)]
struct GoogleResponse {
    pub status: String,
    pub results: Vec<GoogleResult>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleResult {
    pub formatted_address: Option<String>,
    pub geometry: Option<GoogleGeometry>,
}

#[derive(Debug, Deserialize)]
struct GoogleGeometry {
    pub location: Option<GoogleLatLng>,
}

#[derive(Debug, Deserialize)]
struct GoogleLatLng {
    pub lat: f64,
    pub lng: f64,
}

/// Google Maps Geocoder
pub struct GoogleMapsGeocoder {
    client: reqwest::Client,
    api_key: String,
}

impl GoogleMapsGeocoder {
    pub fn new(api_key: String, config: &Config) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_default();

        Self { client, api_key }
    }
}

#[async_trait]
impl Geocoder for GoogleMapsGeocoder {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(GeocodeError::NotFound("Empty query provided".to_string()));
        }

        let resp = self
            .client
            .get("https://maps.googleapis.com/maps/api/geocode/json")
            .query(&[
                ("address", trimmed),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "Google Maps HTTP status: {}",
                resp.status()
            )));
        }

        let data: GoogleResponse = resp.json().await?;
        if data.status == "OVER_QUERY_LIMIT" {
            return Err(GeocodeError::RateLimited);
        }
        if data.status == "ZERO_RESULTS" {
            return Err(GeocodeError::NotFound(trimmed.to_string()));
        }
        if data.status != "OK" {
            return Err(GeocodeError::Service(
                data.error_message.unwrap_or(data.status),
            ));
        }

        let first = data
            .results
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(trimmed.to_string()))?;

        let loc = first
            .geometry
            .and_then(|g| g.location)
            .ok_or_else(|| GeocodeError::ParseError("Missing geometry location in Google response".to_string()))?;

        let address = first.formatted_address.unwrap_or_else(|| trimmed.to_string());
        let name = address
            .split(',')
            .next()
            .unwrap_or(trimmed)
            .trim()
            .to_string();

        Location::new(name, address, loc.lat, loc.lng).map_err(GeocodeError::InvalidCoordinate)
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        Location::validate_coordinates(lat, lon).map_err(GeocodeError::InvalidCoordinate)?;

        let latlng = format!("{:.6},{:.6}", lat, lon);
        let resp = self
            .client
            .get("https://maps.googleapis.com/maps/api/geocode/json")
            .query(&[
                ("latlng", latlng.as_str()),
                ("key", self.api_key.as_str()),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(GeocodeError::Service(format!(
                "Google Maps reverse HTTP status: {}",
                resp.status()
            )));
        }

        let data: GoogleResponse = resp.json().await?;
        if data.status == "OVER_QUERY_LIMIT" {
            return Err(GeocodeError::RateLimited);
        }
        if data.status == "ZERO_RESULTS" {
            return Err(GeocodeError::NotFound(format!("{:.6}, {:.6}", lat, lon)));
        }
        if data.status != "OK" {
            return Err(GeocodeError::Service(
                data.error_message.unwrap_or(data.status),
            ));
        }

        let first = data
            .results
            .into_iter()
            .next()
            .ok_or_else(|| GeocodeError::NotFound(format!("{:.6}, {:.6}", lat, lon)))?;

        let address = first
            .formatted_address
            .unwrap_or_else(|| format!("{:.5}, {:.5}", lat, lon));
        let name = address
            .split(',')
            .next()
            .unwrap_or("Custom Location")
            .trim()
            .to_string();

        Location::new(name, address, lat, lon).map_err(GeocodeError::InvalidCoordinate)
    }
}

#[async_trait]
impl Geocoder for Box<dyn Geocoder> {
    async fn geocode(&self, query: &str) -> Result<Location, GeocodeError> {
        (**self).geocode(query).await
    }

    async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<Location, GeocodeError> {
        (**self).reverse_geocode(lat, lon).await
    }
}

/// Factory function to instantiate the requested geocoder backend.
pub fn create_geocoder(
    provider: GeocoderProvider,
    api_key: Option<String>,
    config: &Config,
) -> Result<Box<dyn Geocoder>, GeocodeError> {
    match provider {
        GeocoderProvider::Nominatim => Ok(Box::new(NominatimGeocoder::new(config.clone()))),
        GeocoderProvider::Locationiq => {
            let key = api_key.or_else(|| config.api_key.clone()).ok_or_else(|| {
                GeocodeError::MissingApiKey("LocationIQ".to_string())
            })?;
            Ok(Box::new(LocationIqGeocoder::new(key, config)))
        }
        GeocoderProvider::Mapbox => {
            let key = api_key.or_else(|| config.api_key.clone()).ok_or_else(|| {
                GeocodeError::MissingApiKey("Mapbox".to_string())
            })?;
            Ok(Box::new(MapboxGeocoder::new(key, config)))
        }
        GeocoderProvider::Opencage => {
            let key = api_key.or_else(|| config.api_key.clone()).ok_or_else(|| {
                GeocodeError::MissingApiKey("OpenCage".to_string())
            })?;
            Ok(Box::new(OpenCageGeocoder::new(key, config)))
        }
        GeocoderProvider::Google => {
            let key = api_key.or_else(|| config.api_key.clone()).ok_or_else(|| {
                GeocodeError::MissingApiKey("Google Maps".to_string())
            })?;
            Ok(Box::new(GoogleMapsGeocoder::new(key, config)))
        }
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
        let config = Config {
            cache_enabled: true,
            ..Config::default()
        };

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

    #[test]
    fn test_create_geocoder_nominatim_no_key_required() {
        let config = Config::default();
        let res = create_geocoder(GeocoderProvider::Nominatim, None, &config);
        assert!(res.is_ok());
    }

    #[test]
    fn test_create_geocoder_missing_api_key() {
        let config = Config::default();
        let res = create_geocoder(GeocoderProvider::Mapbox, None, &config);
        assert!(matches!(res, Err(GeocodeError::MissingApiKey(_))));

        let res2 = create_geocoder(GeocoderProvider::Google, None, &config);
        assert!(matches!(res2, Err(GeocodeError::MissingApiKey(_))));
    }

    #[test]
    fn test_create_geocoder_with_api_key() {
        let config = Config::default();
        let res = create_geocoder(GeocoderProvider::Mapbox, Some("pk.test123".to_string()), &config);
        assert!(res.is_ok());

        let res2 = create_geocoder(GeocoderProvider::Locationiq, Some("iq_test_key".to_string()), &config);
        assert!(res2.is_ok());
    }
}


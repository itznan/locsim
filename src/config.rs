use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Supported geocoder backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum, Default)]
#[serde(rename_all = "lowercase")]
pub enum GeocoderProvider {
    #[default]
    Nominatim,
    Locationiq,
    Mapbox,
    Opencage,
    Google,
}

impl std::fmt::Display for GeocoderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nominatim => write!(f, "nominatim"),
            Self::Locationiq => write!(f, "locationiq"),
            Self::Mapbox => write!(f, "mapbox"),
            Self::Opencage => write!(f, "opencage"),
            Self::Google => write!(f, "google"),
        }
    }
}

impl std::str::FromStr for GeocoderProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "nominatim" | "osm" => Ok(Self::Nominatim),
            "locationiq" | "iq" => Ok(Self::Locationiq),
            "mapbox" => Ok(Self::Mapbox),
            "opencage" | "cage" => Ok(Self::Opencage),
            "google" | "googlemaps" => Ok(Self::Google),
            _ => Err(format!("Unknown geocoder provider: {}", s)),
        }
    }
}

/// Optional file-based configuration layout (TOML)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub cache_enabled: Option<bool>,
    pub timeout_seconds: Option<u64>,
    pub user_agent: Option<String>,
    pub geocoder_url: Option<String>,
    pub reverse_geocoder_url: Option<String>,
}

/// Application configuration settings for locsim.
#[derive(Debug, Clone)]
pub struct Config {
    pub provider: GeocoderProvider,
    pub api_key: Option<String>,
    pub geocoder_url: String,
    pub reverse_geocoder_url: String,
    pub user_agent: String,
    pub cache_enabled: bool,
    pub timeout_seconds: u64,
}

impl Config {
    /// Load configuration following precedence:
    /// Defaults < Global Config (~/.config/locsim/config.toml) < Local Config (./.locsimrc) < Environment Variables
    pub fn load() -> Self {
        let mut config = Self::default_values();

        // 1. Try global config file
        let global_config = AppPaths::global_config_file();
        if global_config.exists() {
            if let Ok(content) = fs::read_to_string(&global_config) {
                if let Ok(parsed) = toml::from_str::<ConfigFile>(&content) {
                    config.apply_file(&parsed);
                }
            }
        }

        // 2. Try local project config file (.locsim.toml or .locsimrc)
        if let Some(local_path) = AppPaths::local_config_file() {
            if let Ok(content) = fs::read_to_string(&local_path) {
                if let Ok(parsed) = toml::from_str::<ConfigFile>(&content) {
                    config.apply_file(&parsed);
                }
            }
        }

        // 3. Environment variables override config files
        config.apply_env_vars();

        config
    }

    fn default_values() -> Self {
        Self {
            provider: GeocoderProvider::Nominatim,
            api_key: None,
            geocoder_url: "https://nominatim.openstreetmap.org/search".to_string(),
            reverse_geocoder_url: "https://nominatim.openstreetmap.org/reverse".to_string(),
            user_agent: "locsim/0.1.0 (https://github.com/locsim/locsim; universal-location-simulator)".to_string(),
            cache_enabled: true,
            timeout_seconds: 10,
        }
    }

    fn apply_file(&mut self, file: &ConfigFile) {
        if let Some(ref p) = file.provider {
            if let Ok(provider) = p.parse::<GeocoderProvider>() {
                self.provider = provider;
            }
        }
        if let Some(ref key) = file.api_key {
            self.api_key = Some(key.clone());
        }
        if let Some(ref url) = file.geocoder_url {
            self.geocoder_url = url.clone();
        }
        if let Some(ref r_url) = file.reverse_geocoder_url {
            self.reverse_geocoder_url = r_url.clone();
        }
        if let Some(ref ua) = file.user_agent {
            self.user_agent = ua.clone();
        }
        if let Some(cache) = file.cache_enabled {
            self.cache_enabled = cache;
        }
        if let Some(timeout) = file.timeout_seconds {
            self.timeout_seconds = timeout;
        }
    }

    fn apply_env_vars(&mut self) {
        if let Ok(val) = std::env::var("LOCSIM_PROVIDER") {
            if let Ok(provider) = val.parse::<GeocoderProvider>() {
                self.provider = provider;
            }
        }

        if let Ok(key) = std::env::var("LOCSIM_API_KEY")
            .or_else(|_| std::env::var("LOCATIONIQ_API_KEY"))
            .or_else(|_| std::env::var("MAPBOX_ACCESS_TOKEN"))
            .or_else(|_| std::env::var("OPENCAGE_API_KEY"))
            .or_else(|_| std::env::var("GOOGLE_MAPS_API_KEY"))
        {
            if !key.trim().is_empty() {
                self.api_key = Some(key);
            }
        }

        if let Ok(url) = std::env::var("LOCSIM_GEOCODER_URL") {
            self.geocoder_url = url;
        }

        if let Ok(url) = std::env::var("LOCSIM_REVERSE_GEOCODER_URL") {
            self.reverse_geocoder_url = url;
        }

        if let Ok(ua) = std::env::var("LOCSIM_USER_AGENT") {
            self.user_agent = ua;
        }

        if let Ok(v) = std::env::var("LOCSIM_CACHE_ENABLED") {
            self.cache_enabled = v != "0" && v.to_lowercase() != "false";
        }

        if let Ok(t) = std::env::var("LOCSIM_TIMEOUT") {
            if let Ok(parsed) = t.parse::<u64>() {
                self.timeout_seconds = parsed;
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::load()
    }
}


/// Helper methods to locate locsim configuration, state, and export files.
pub struct AppPaths;

impl AppPaths {
    /// Return the base data/configuration directory for locsim.
    pub fn config_dir() -> PathBuf {
        if let Ok(custom) = std::env::var("LOCSIM_DATA_DIR") {
            let path = PathBuf::from(custom);
            let _ = fs::create_dir_all(&path);
            return path;
        }

        if let Some(proj_dirs) = ProjectDirs::from("com", "locsim", "locsim") {
            let path = proj_dirs.config_dir().to_path_buf();
            let _ = fs::create_dir_all(&path);
            path
        } else {
            let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            path.push(".locsim");
            let _ = fs::create_dir_all(&path);
            path
        }
    }

    /// Path to global configuration file (~/.config/locsim/config.toml)
    pub fn global_config_file() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Check for project-local config file (.locsim.toml or .locsimrc) in current directory
    pub fn local_config_file() -> Option<PathBuf> {
        let candidates = [".locsim.toml", ".locsimrc", "locsim.toml"];
        for c in &candidates {
            let p = Path::new(c);
            if p.exists() && p.is_file() {
                return Some(p.to_path_buf());
            }
        }
        None
    }

    /// Path to current simulated location JSON file.
    pub fn current_location_file() -> PathBuf {
        Self::config_dir().join("current_location.json")
    }

    /// Path to geocoding cache JSON file.
    pub fn cache_file() -> PathBuf {
        Self::config_dir().join("geocode_cache.json")
    }

    /// Path to GPX simulated track/waypoint file for simulators and IDEs.
    pub fn gpx_file() -> PathBuf {
        Self::config_dir().join("simulated_location.gpx")
    }

    /// Path to Chrome DevTools Protocol (CDP) geolocation override JSON.
    pub fn cdp_file() -> PathBuf {
        Self::config_dir().join("cdp_geolocation.json")
    }

    /// Path to bookmarks/saved locations JSON file.
    pub fn bookmarks_file() -> PathBuf {
        Self::config_dir().join("bookmarks.json")
    }

    /// Path to environment variable script (PowerShell or Bash).
    pub fn env_script_file() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            Self::config_dir().join("locsim_env.ps1")
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::config_dir().join("locsim_env.sh")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.provider, GeocoderProvider::Nominatim);
        assert!(config.geocoder_url.contains("nominatim"));
        assert!(config.user_agent.contains("locsim"));
        assert!(config.cache_enabled);
        assert_eq!(config.timeout_seconds, 10);
    }

    #[test]
    fn test_app_paths_creation() {
        let dir = AppPaths::config_dir();
        assert!(dir.exists(), "Config directory should exist");
        let loc_file = AppPaths::current_location_file();
        assert_eq!(loc_file.file_name().unwrap(), "current_location.json");
        let global_cfg = AppPaths::global_config_file();
        assert_eq!(global_cfg.file_name().unwrap(), "config.toml");
    }

    #[test]
    fn test_config_from_toml_file() {
        let toml_str = r#"
            provider = "mapbox"
            api_key = "pk.test_12345"
            cache_enabled = false
            timeout_seconds = 25
            user_agent = "custom-agent/2.0"
        "#;

        let parsed: ConfigFile = toml::from_str(toml_str).unwrap();
        let mut config = Config::default_values();
        config.apply_file(&parsed);

        assert_eq!(config.provider, GeocoderProvider::Mapbox);
        assert_eq!(config.api_key.as_deref(), Some("pk.test_12345"));
        assert!(!config.cache_enabled);
        assert_eq!(config.timeout_seconds, 25);
        assert_eq!(config.user_agent, "custom-agent/2.0");
    }

    #[test]
    fn test_geocoder_provider_parsing() {
        assert_eq!("nominatim".parse::<GeocoderProvider>().unwrap(), GeocoderProvider::Nominatim);
        assert_eq!("osm".parse::<GeocoderProvider>().unwrap(), GeocoderProvider::Nominatim);
        assert_eq!("locationiq".parse::<GeocoderProvider>().unwrap(), GeocoderProvider::Locationiq);
        assert_eq!("mapbox".parse::<GeocoderProvider>().unwrap(), GeocoderProvider::Mapbox);
        assert_eq!("opencage".parse::<GeocoderProvider>().unwrap(), GeocoderProvider::Opencage);
        assert_eq!("google".parse::<GeocoderProvider>().unwrap(), GeocoderProvider::Google);
        assert!("invalid".parse::<GeocoderProvider>().is_err());
    }
}


use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

/// Application configuration settings for locsim.
#[derive(Debug, Clone)]
pub struct Config {
    pub geocoder_url: String,
    pub reverse_geocoder_url: String,
    pub user_agent: String,
    pub cache_enabled: bool,
    pub timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        let geocoder_url = std::env::var("LOCSIM_GEOCODER_URL")
            .unwrap_or_else(|_| "https://nominatim.openstreetmap.org/search".to_string());

        let reverse_geocoder_url = std::env::var("LOCSIM_REVERSE_GEOCODER_URL")
            .unwrap_or_else(|_| "https://nominatim.openstreetmap.org/reverse".to_string());

        let user_agent = std::env::var("LOCSIM_USER_AGENT")
            .unwrap_or_else(|_| "locsim/0.1.0 (https://github.com/locsim/locsim; universal-location-simulator)".to_string());

        let cache_enabled = std::env::var("LOCSIM_CACHE_ENABLED")
            .map(|v| v != "0" && v.to_lowercase() != "false")
            .unwrap_or(true);

        let timeout_seconds = std::env::var("LOCSIM_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        Self {
            geocoder_url,
            reverse_geocoder_url,
            user_agent,
            cache_enabled,
            timeout_seconds,
        }
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
    }
}

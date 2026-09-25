use crate::location::Location;
use std::path::PathBuf;

pub mod android;
pub mod linux;
pub mod macos;
pub mod windows;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum LocationError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("OS platform error: {0}")]
    Platform(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),
}

/// Metadata diagnostics describing how the platform provides simulated locations.
#[derive(Debug, Clone)]
pub struct ProviderDiagnostics {
    pub platform_name: &'static str,
    pub mechanism_name: &'static str,
    pub os_mechanism_description: &'static str,
    pub browser_integration_guide: &'static str,
    pub exported_files: Vec<PathBuf>,
    pub is_elevated: bool,
    pub elevation_note: Option<String>,
}

/// The core platform abstraction trait for setting, retrieving, and clearing simulated locations.
pub trait LocationProvider: Send + Sync {
    /// Applies the simulated location using the platform's documented mechanisms.
    fn set_location(&self, location: &Location) -> Result<(), LocationError>;

    /// Retrieves the currently configured simulated location, if any.
    fn get_location(&self) -> Result<Option<Location>, LocationError>;

    /// Clears the simulated location and resets mock overrides.
    fn clear_location(&self) -> Result<(), LocationError>;

    /// Returns the user-friendly name of this location provider.
    fn provider_name(&self) -> &str;

    /// Returns diagnostic information about OS and browser simulation support.
    fn diagnostics(&self) -> ProviderDiagnostics;
}

/// Factory function to obtain the appropriate provider for the current operating system.
pub fn get_platform_provider() -> Box<dyn LocationProvider> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsLocationProvider::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxLocationProvider::new())
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsLocationProvider::new())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Box::new(generic_provider::GenericLocationProvider::new())
    }
}

/// Helper to generate a GPX waypoint XML file for GPS simulators, Xcode, and mapping tools.
pub fn generate_gpx_content(location: &Location) -> String {
    let mut extra_tags = String::new();
    if let Some(alt) = location.altitude {
        extra_tags.push_str(&format!("    <ele>{:.2}</ele>\n", alt));
    }
    if let Some(spd) = location.speed {
        extra_tags.push_str(&format!("    <speed>{:.2}</speed>\n", spd));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="locsim" xmlns="http://www.topografix.com/GPX/1/1">
  <wpt lat="{lat:.6}" lon="{lon:.6}">
{extra_tags}    <name>{name}</name>
    <desc>{desc}</desc>
  </wpt>
</gpx>
"#,
        lat = location.latitude,
        lon = location.longitude,
        extra_tags = extra_tags,
        name = quick_xml_escape(&location.name),
        desc = quick_xml_escape(&location.address)
    )
}

/// Helper to generate Chrome DevTools Protocol (CDP) JSON payload for browser geolocation emulation.
pub fn generate_cdp_payload(location: &Location) -> String {
    let mut params = serde_json::json!({
        "latitude": location.latitude,
        "longitude": location.longitude,
        "accuracy": location.accuracy.unwrap_or(100.0),
    });

    if let Some(alt) = location.altitude {
        params["altitude"] = serde_json::json!(alt);
    }
    if let Some(spd) = location.speed {
        params["speed"] = serde_json::json!(spd);
    }
    if let Some(hdg) = location.heading {
        params["heading"] = serde_json::json!(hdg);
    }

    serde_json::json!({
        "method": "Emulation.setGeolocationOverride",
        "params": params
    })
    .to_string()
}

fn quick_xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpx_generation() {
        let mut loc = Location::new("Test Point", "Test Address <>&", 12.345678, 98.765432).unwrap();
        loc.altitude = Some(450.5);
        loc.speed = Some(15.2);
        let gpx = generate_gpx_content(&loc);
        assert!(gpx.contains(r#"lat="12.345678""#));
        assert!(gpx.contains(r#"lon="98.765432""#));
        assert!(gpx.contains("<name>Test Point</name>"));
        assert!(gpx.contains("&lt;&gt;&amp;"));
        assert!(gpx.contains("<ele>450.50</ele>"));
        assert!(gpx.contains("<speed>15.20</speed>"));
    }

    #[test]
    fn test_cdp_generation() {
        let mut loc = Location::new("CDP Point", "Some address", 20.0, 30.0).unwrap();
        loc.accuracy = Some(15.0);
        loc.altitude = Some(200.0);
        loc.heading = Some(90.0);
        loc.speed = Some(10.0);

        let cdp = generate_cdp_payload(&loc);
        assert!(cdp.contains("Emulation.setGeolocationOverride"));
        assert!(cdp.contains(r#""latitude":20.0"#));
        assert!(cdp.contains(r#""longitude":30.0"#));
        assert!(cdp.contains(r#""accuracy":15.0"#));
        assert!(cdp.contains(r#""altitude":200.0"#));
        assert!(cdp.contains(r#""heading":90.0"#));
        assert!(cdp.contains(r#""speed":10.0"#));
    }
}

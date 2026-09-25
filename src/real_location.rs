use serde::Deserialize;
use std::time::Duration;

/// Holds IP-based geolocation data for real device location detection.
#[derive(Debug, Clone, Deserialize)]
pub struct RealLocationInfo {
    #[serde(rename = "query")]
    pub ip: Option<String>,
    pub city: Option<String>,
    #[serde(rename = "regionName")]
    pub region: Option<String>,
    pub country: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub isp: Option<String>,
}

/// Detects the real physical location of the machine via public IP geolocation.
pub async fn fetch_real_location() -> Result<RealLocationInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let resp = client
        .get("http://ip-api.com/json/")
        .send()
        .await
        .map_err(|e| format!("Could not reach IP geolocation service: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Geolocation service returned status {}", resp.status()));
    }

    let info: RealLocationInfo = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse IP geolocation response: {}", e))?;

    Ok(info)
}

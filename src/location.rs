use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents an error in location coordinate validation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LocationValidationError {
    #[error("Latitude {0} is out of valid range [-90.0, 90.0]")]
    InvalidLatitude(f64),

    #[error("Longitude {0} is out of valid range [-180.0, 180.0]")]
    InvalidLongitude(f64),

    #[error("Coordinates must be finite numbers (cannot be NaN or Infinite)")]
    NonFiniteCoordinate,

    #[error("Altitude must be a finite number (cannot be NaN or Infinite)")]
    NonFiniteAltitude,

    #[error("Accuracy must be a non-negative finite number (got {0})")]
    InvalidAccuracy(f64),

    #[error("Speed must be a non-negative finite number (got {0})")]
    InvalidSpeed(f64),

    #[error("Heading must be between 0.0 and 360.0 degrees (got {0})")]
    InvalidHeading(f64),
}

/// Represents a validated geographical location with descriptive metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub altitude: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<f64>,
}

impl Location {
    /// Create a new Location with validated coordinates (without telemetry).
    pub fn new(
        name: impl Into<String>,
        address: impl Into<String>,
        latitude: f64,
        longitude: f64,
    ) -> Result<Self, LocationValidationError> {
        Self::new_with_telemetry(name, address, latitude, longitude, None, None, None, None)
    }

    /// Create a new Location with validated coordinates and optional telemetry fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_telemetry(
        name: impl Into<String>,
        address: impl Into<String>,
        latitude: f64,
        longitude: f64,
        altitude: Option<f64>,
        accuracy: Option<f64>,
        speed: Option<f64>,
        heading: Option<f64>,
    ) -> Result<Self, LocationValidationError> {
        Self::validate_coordinates(latitude, longitude)?;
        Self::validate_telemetry(altitude, accuracy, speed, heading)?;

        let name_str = name.into();
        let address_str = address.into();

        Ok(Self {
            name: if name_str.trim().is_empty() {
                format!("{:.5}, {:.5}", latitude, longitude)
            } else {
                name_str.trim().to_string()
            },
            address: if address_str.trim().is_empty() {
                format!("Coordinates: {:.5}, {:.5}", latitude, longitude)
            } else {
                address_str.trim().to_string()
            },
            latitude,
            longitude,
            altitude,
            accuracy,
            speed,
            heading,
        })
    }

    /// Validate telemetry fields if present.
    pub fn validate_telemetry(
        altitude: Option<f64>,
        accuracy: Option<f64>,
        speed: Option<f64>,
        heading: Option<f64>,
    ) -> Result<(), LocationValidationError> {
        if let Some(alt) = altitude {
            if alt.is_nan() || alt.is_infinite() {
                return Err(LocationValidationError::NonFiniteAltitude);
            }
        }

        if let Some(acc) = accuracy {
            if acc.is_nan() || acc.is_infinite() || acc < 0.0 {
                return Err(LocationValidationError::InvalidAccuracy(acc));
            }
        }

        if let Some(spd) = speed {
            if spd.is_nan() || spd.is_infinite() || spd < 0.0 {
                return Err(LocationValidationError::InvalidSpeed(spd));
            }
        }

        if let Some(hdg) = heading {
            if hdg.is_nan() || hdg.is_infinite() || !(0.0..=360.0).contains(&hdg) {
                return Err(LocationValidationError::InvalidHeading(hdg));
            }
        }

        Ok(())
    }

    /// Builder method to attach or update altitude in meters.
    #[allow(dead_code)]
    pub fn with_altitude(mut self, altitude: Option<f64>) -> Result<Self, LocationValidationError> {
        Self::validate_telemetry(altitude, None, None, None)?;
        self.altitude = altitude;
        Ok(self)
    }

    /// Builder method to attach or update horizontal accuracy radius in meters.
    #[allow(dead_code)]
    pub fn with_accuracy(mut self, accuracy: Option<f64>) -> Result<Self, LocationValidationError> {
        Self::validate_telemetry(None, accuracy, None, None)?;
        self.accuracy = accuracy;
        Ok(self)
    }

    /// Builder method to attach or update speed in meters per second.
    #[allow(dead_code)]
    pub fn with_speed(mut self, speed: Option<f64>) -> Result<Self, LocationValidationError> {
        Self::validate_telemetry(None, None, speed, None)?;
        self.speed = speed;
        Ok(self)
    }

    /// Builder method to attach or update heading in degrees [0, 360].
    #[allow(dead_code)]
    pub fn with_heading(mut self, heading: Option<f64>) -> Result<Self, LocationValidationError> {
        Self::validate_telemetry(None, None, None, heading)?;
        self.heading = heading;
        Ok(self)
    }

    /// Validate latitude and longitude bounds:
    /// -90.0 <= latitude <= 90.0
    /// -180.0 <= longitude <= 180.0
    pub fn validate_coordinates(latitude: f64, longitude: f64) -> Result<(), LocationValidationError> {
        if latitude.is_nan() || latitude.is_infinite() || longitude.is_nan() || longitude.is_infinite() {
            return Err(LocationValidationError::NonFiniteCoordinate);
        }

        if !(-90.0..=90.0).contains(&latitude) {
            return Err(LocationValidationError::InvalidLatitude(latitude));
        }

        if !(-180.0..=180.0).contains(&longitude) {
            return Err(LocationValidationError::InvalidLongitude(longitude));
        }

        Ok(())
    }

    /// Format the address with indent wrapping for clean CLI presentation.
    pub fn formatted_address(&self, indent_spaces: usize) -> String {
        let indent = " ".repeat(indent_spaces);
        let max_line_len = 50;
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for part in self.address.split(',') {
            let item = part.trim();
            if item.is_empty() {
                continue;
            }

            if current_line.is_empty() {
                current_line.push_str(item);
            } else if current_line.len() + item.len() + 2 <= max_line_len {
                current_line.push_str(", ");
                current_line.push_str(item);
            } else {
                lines.push(current_line);
                current_line = item.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            return self.address.clone();
        }

        lines.join(&format!("\n{}", indent))
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({:.6}, {:.6}) - {}",
            self.name, self.latitude, self.longitude, self.address
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_coordinates() {
        assert!(Location::validate_coordinates(0.0, 0.0).is_ok());
        assert!(Location::validate_coordinates(90.0, 180.0).is_ok());
        assert!(Location::validate_coordinates(-90.0, -180.0).is_ok());
        assert!(Location::validate_coordinates(22.3072, 73.1812).is_ok());
        assert!(Location::validate_coordinates(-33.8688, 151.2093).is_ok());
    }

    #[test]
    fn test_invalid_latitude() {
        assert_eq!(
            Location::validate_coordinates(90.0001, 0.0),
            Err(LocationValidationError::InvalidLatitude(90.0001))
        );
        assert_eq!(
            Location::validate_coordinates(-90.1, 0.0),
            Err(LocationValidationError::InvalidLatitude(-90.1))
        );
    }

    #[test]
    fn test_invalid_longitude() {
        assert_eq!(
            Location::validate_coordinates(0.0, 180.0001),
            Err(LocationValidationError::InvalidLongitude(180.0001))
        );
        assert_eq!(
            Location::validate_coordinates(0.0, -180.1),
            Err(LocationValidationError::InvalidLongitude(-180.1))
        );
    }

    #[test]
    fn test_non_finite_coordinates() {
        assert_eq!(
            Location::validate_coordinates(f64::NAN, 0.0),
            Err(LocationValidationError::NonFiniteCoordinate)
        );
        assert_eq!(
            Location::validate_coordinates(0.0, f64::INFINITY),
            Err(LocationValidationError::NonFiniteCoordinate)
        );
        assert_eq!(
            Location::validate_coordinates(f64::NEG_INFINITY, 0.0),
            Err(LocationValidationError::NonFiniteCoordinate)
        );
    }

    #[test]
    fn test_location_creation_and_defaults() {
        let loc = Location::new(
            "Marwadi University",
            "Gauridad, Rajkot, Gujarat, India",
            22.3688,
            70.8022,
        )
        .expect("Valid location should be created");

        assert_eq!(loc.name, "Marwadi University");
        assert_eq!(loc.latitude, 22.3688);
        assert_eq!(loc.longitude, 70.8022);

        // Blank name/address fallback to coordinates
        let fallback = Location::new("", "", 10.0, 20.0).expect("Fallback location");
        assert_eq!(fallback.name, "10.00000, 20.00000");
    }

    #[test]
    fn test_location_json_serialization() {
        let loc = Location::new("Test", "Test Address", 12.34, 56.78).unwrap();
        let json = serde_json::to_string(&loc).expect("Serialize");
        let deserialized: Location = serde_json::from_str(&json).expect("Deserialize");
        assert_eq!(loc, deserialized);
        assert_eq!(deserialized.altitude, None);
    }

    #[test]
    fn test_location_telemetry_validations_and_builder() {
        let loc = Location::new("Tele Test", "Address", 10.0, 20.0)
            .unwrap()
            .with_altitude(Some(150.5))
            .unwrap()
            .with_accuracy(Some(5.0))
            .unwrap()
            .with_speed(Some(12.2))
            .unwrap()
            .with_heading(Some(180.0))
            .unwrap();

        assert_eq!(loc.altitude, Some(150.5));
        assert_eq!(loc.accuracy, Some(5.0));
        assert_eq!(loc.speed, Some(12.2));
        assert_eq!(loc.heading, Some(180.0));

        // Invalid accuracy (negative)
        assert!(loc.clone().with_accuracy(Some(-1.0)).is_err());
        // Invalid speed (negative)
        assert!(loc.clone().with_speed(Some(-0.5)).is_err());
        // Invalid heading (> 360.0)
        assert!(loc.clone().with_heading(Some(361.0)).is_err());
        // Non-finite altitude
        assert!(loc.clone().with_altitude(Some(f64::NAN)).is_err());
    }

    #[test]
    fn test_location_telemetry_json_roundtrip() {
        let loc = Location::new_with_telemetry(
            "Tele",
            "Addr",
            30.0,
            40.0,
            Some(100.0),
            Some(10.0),
            Some(5.0),
            Some(90.0),
        )
        .unwrap();

        let json = serde_json::to_string(&loc).expect("Serialize");
        assert!(json.contains(r#""altitude":100.0"#));
        assert!(json.contains(r#""accuracy":10.0"#));
        let deserialized: Location = serde_json::from_str(&json).expect("Deserialize");
        assert_eq!(loc, deserialized);
    }
}

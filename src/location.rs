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
}

/// Represents a validated geographical location with descriptive metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub address: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    /// Create a new Location with validated coordinates.
    pub fn new(
        name: impl Into<String>,
        address: impl Into<String>,
        latitude: f64,
        longitude: f64,
    ) -> Result<Self, LocationValidationError> {
        Self::validate_coordinates(latitude, longitude)?;

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
        })
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
    }
}

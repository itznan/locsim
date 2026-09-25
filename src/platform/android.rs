use crate::location::Location;
use std::process::Command;

/// Diagnostics and device info for Android ADB Emulator location bridging.
#[derive(Debug, Clone, PartialEq)]
pub struct AndroidStatus {
    pub adb_available: bool,
    pub adb_version: Option<String>,
    pub running_emulators: Vec<String>,
}

/// Provides direct integration with Android emulators via the Android Debug Bridge (ADB).
pub struct AndroidBridge;

impl AndroidBridge {
    /// Detects if `adb` is present in the system PATH and queries connected emulators.
    pub fn status() -> AndroidStatus {
        let (adb_available, adb_version) = match Command::new("adb").arg("--version").output() {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let first_line = stdout.lines().next().unwrap_or("ADB").trim().to_string();
                (true, Some(first_line))
            }
            _ => (false, None),
        };

        let running_emulators = if adb_available {
            Self::list_emulators()
        } else {
            Vec::new()
        };

        AndroidStatus {
            adb_available,
            adb_version,
            running_emulators,
        }
    }

    /// Lists connected Android devices/emulators via `adb devices`.
    pub fn list_emulators() -> Vec<String> {
        let output = match Command::new("adb").arg("devices").output() {
            Ok(o) => o,
            Err(_) => return Vec::new(),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_adb_devices(&stdout)
    }

    /// Parses the raw stdout of `adb devices`.
    pub fn parse_adb_devices(stdout: &str) -> Vec<String> {
        let mut devices = Vec::new();
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && (parts[1] == "device" || parts[1] == "emulator") {
                devices.push(parts[0].to_string());
            }
        }
        devices
    }

    /// Sends a geo fix to a specific emulator or all connected emulators.
    /// Syntax: `adb -s <serial> emu geo fix <longitude> <latitude> [altitude]`
    pub fn send_geo_fix(serial: Option<&str>, location: &Location) -> Result<Vec<String>, String> {
        let targets = match serial {
            Some(s) => vec![s.to_string()],
            None => Self::list_emulators(),
        };

        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let mut applied_to = Vec::new();

        for target in &targets {
            let mut args = vec![
                "-s",
                target.as_str(),
                "emu",
                "geo",
                "fix",
            ];

            let lon_str = format!("{:.6}", location.longitude);
            let lat_str = format!("{:.6}", location.latitude);
            args.push(&lon_str);
            args.push(&lat_str);

            let alt_str;
            if let Some(alt) = location.altitude {
                alt_str = format!("{:.1}", alt);
                args.push(&alt_str);
            }

            match Command::new("adb").args(&args).output() {
                Ok(output) if output.status.success() => {
                    applied_to.push(target.clone());
                }
                Ok(_) => {
                    // Also attempt modern Android 10+ set-location fallback
                    let _ = Command::new("adb")
                        .args(["-s", target.as_str(), "shell", "cmd", "location", "set-location", &lat_str, &lon_str])
                        .output();
                    applied_to.push(target.clone());
                }
                Err(e) => {
                    return Err(format!("Failed to execute adb command: {}", e));
                }
            }
        }

        Ok(applied_to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adb_devices() {
        let sample = "List of devices attached\nemulator-5554\tdevice\nemulator-5556\tdevice\noffline-device\toffline\n";
        let parsed = AndroidBridge::parse_adb_devices(sample);
        assert_eq!(parsed, vec!["emulator-5554", "emulator-5556"]);
    }

    #[test]
    fn test_parse_adb_empty() {
        let sample = "List of devices attached\n\n";
        let parsed = AndroidBridge::parse_adb_devices(sample);
        assert!(parsed.is_empty());
    }
}

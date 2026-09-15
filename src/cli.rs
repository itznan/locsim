use clap::Parser;

/// Universal CLI-based location simulator in Rust.
///
/// Easily mock your device location for developer testing, simulators,
/// and location-aware applications across Windows, Linux, and macOS.
#[derive(Parser, Debug)]
#[command(
    name = "locsim",
    author = "locsim contributors",
    version = "0.1.0",
    about = "Universal CLI-based location simulator in Rust",
    long_about = "Universal CLI-based location simulator in Rust.\n\n\
                  Resolves location names to coordinates, verifies inputs, and configures\n\
                  documented OS-supported mock location and developer test mechanisms."
)]
pub struct Cli {
    /// Location search query (e.g. "Marwadi University, Rajkot", "Mumbai, India")
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Set manual latitude (-90.0 to 90.0)
    #[arg(long, value_name = "LATITUDE", allow_hyphen_values = true)]
    pub lat: Option<f64>,

    /// Set manual longitude (-180.0 to 180.0)
    #[arg(long, value_name = "LONGITUDE", allow_hyphen_values = true)]
    pub lon: Option<f64>,

    /// Display the currently configured simulated location and diagnostics
    #[arg(long, short = 's')]
    pub show: bool,

    /// Clear the simulated location and reset overrides
    #[arg(long, short = 'c')]
    pub clear: bool,

    /// Launch interactive menu mode
    #[arg(long, short = 'i')]
    pub interactive: bool,

    /// Automatically accept confirmation prompts without prompting
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Custom Geocoder API URL (overrides default Nominatim)
    #[arg(long, env = "LOCSIM_GEOCODER_URL")]
    pub geocoder_url: Option<String>,
}

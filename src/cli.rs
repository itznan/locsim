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
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Location search query (e.g. "Marwadi University, Rajkot", "Mumbai, India")
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Set manual latitude (-90.0 to 90.0)
    #[arg(long, value_name = "LATITUDE", allow_hyphen_values = true)]
    pub lat: Option<f64>,

    /// Set manual longitude (-180.0 to 180.0)
    #[arg(long, value_name = "LONGITUDE", allow_hyphen_values = true)]
    pub lon: Option<f64>,

    /// Reverse geocode coordinates to an address (e.g. -r 37.7749 -122.4194)
    #[arg(
        long,
        short = 'r',
        num_args = 2,
        value_names = ["LATITUDE", "LONGITUDE"],
        allow_hyphen_values = true
    )]
    pub reverse: Option<Vec<f64>>,

    /// Altitude in meters above sea level (e.g. --altitude 50.0)
    #[arg(long, value_name = "METERS", allow_hyphen_values = true)]
    pub altitude: Option<f64>,

    /// Horizontal accuracy radius in meters (e.g. --accuracy 10.0)
    #[arg(long, value_name = "METERS", allow_hyphen_values = true)]
    pub accuracy: Option<f64>,

    /// Ground speed in meters per second (e.g. --speed 5.5)
    #[arg(long, value_name = "M/S", allow_hyphen_values = true)]
    pub speed: Option<f64>,

    /// Heading / bearing in degrees 0.0 to 360.0 (e.g. --heading 180.0)
    #[arg(long, value_name = "DEGREES", allow_hyphen_values = true)]
    pub heading: Option<f64>,

    /// Display the currently configured simulated location and diagnostics
    #[arg(long, short = 's')]
    pub show: bool,

    /// Clear the simulated location and reset overrides
    #[arg(long, short = 'c')]
    pub clear: bool,

    /// Save current simulated location or query as a named bookmark
    #[arg(long, value_name = "NAME")]
    pub save: Option<String>,

    /// Apply a saved bookmark by name
    #[arg(long, short = 'u', value_name = "NAME")]
    pub r#use: Option<String>,

    /// List all saved location bookmarks
    #[arg(long)]
    pub bookmarks: bool,

    /// Delete a saved bookmark by name
    #[arg(long, value_name = "NAME")]
    pub delete_bookmark: Option<String>,

    /// Reset all simulation and detect real physical IP-based location
    #[arg(long)]
    pub reset: bool,

    /// Launch interactive menu mode
    #[arg(long, short = 'i')]
    pub interactive: bool,

    /// Automatically accept confirmation prompts without prompting
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Target a specific Android emulator device serial (e.g. emulator-5554)
    #[arg(long, value_name = "SERIAL")]
    pub android_device: Option<String>,

    /// Generate shell auto-completion script (bash, zsh, fish, powershell, elvish)
    #[arg(long, value_enum, value_name = "SHELL")]
    pub completions: Option<clap_complete::Shell>,

    /// Geocoder backend provider (nominatim, locationiq, mapbox, opencage, google)
    #[arg(long, short = 'P', value_name = "PROVIDER", value_enum)]
    pub provider: Option<crate::config::GeocoderProvider>,

    /// API key or access token for the selected geocoder provider
    #[arg(long, short = 'K', value_name = "KEY", env = "LOCSIM_API_KEY")]
    pub api_key: Option<String>,

    /// Custom Geocoder API URL (overrides default Nominatim)
    #[arg(long, env = "LOCSIM_GEOCODER_URL")]
    pub geocoder_url: Option<String>,
}


#[derive(clap::Subcommand, Debug, Clone, PartialEq)]
pub enum Commands {
    /// Save current simulated location or query as a named bookmark
    Save {
        /// Bookmark profile name (e.g. "home", "work", "cafe")
        name: String,
        /// Optional query or address to geocode and save directly
        #[arg(value_name = "QUERY")]
        query: Option<String>,
    },

    /// Apply a saved bookmark as the simulated location
    Use {
        /// Bookmark profile name to apply
        name: String,
    },

    /// List all saved location bookmarks
    List,

    /// Delete a saved bookmark
    Delete {
        /// Name of the bookmark to delete
        name: String,
    },

    /// Reset simulated location and detect real physical IP-based location
    Reset,

    /// Generate shell auto-completion script
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

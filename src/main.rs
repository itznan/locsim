use clap::Parser;
use colored::*;
use std::io::{self, Write};

mod cli;
mod config;
mod geocoder;
mod location;
mod platform;

use cli::Cli;
use config::Config;
use geocoder::{CachedGeocoder, Geocoder, NominatimGeocoder};
use location::Location;
use platform::{get_platform_provider, LocationProvider};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mut config = Config::default();

    if let Some(ref custom_url) = cli.geocoder_url {
        config.geocoder_url = custom_url.clone();
    }

    let geocoder = CachedGeocoder::new(NominatimGeocoder::new(config), &Config::default());
    let provider = get_platform_provider();

    // 1. locsim --show
    if cli.show {
        show_current_location(provider.as_ref());
        return;
    }

    // 2. locsim --clear
    if cli.clear {
        clear_location(provider.as_ref());
        return;
    }

    // 3. locsim --interactive / -i
    if cli.interactive {
        run_interactive_menu(&geocoder, provider.as_ref()).await;
        return;
    }

    // 4. locsim --lat <LAT> --lon <LON>
    if cli.lat.is_some() || cli.lon.is_some() {
        match (cli.lat, cli.lon) {
            (Some(lat), Some(lon)) => {
                handle_manual_coordinates(&geocoder, provider.as_ref(), lat, lon, cli.yes).await;
            }
            _ => {
                eprintln!(
                    "{}",
                    "Error: Both --lat and --lon must be provided together.".red().bold()
                );
                std::process::exit(1);
            }
        }
        return;
    }

    // 5. locsim "query"
    if let Some(ref query) = cli.query {
        handle_search_query(&geocoder, provider.as_ref(), query, cli.yes).await;
        return;
    }

    // 6. locsim (default no args) -> Quick Interactive Search Flow
    print_banner();
    run_direct_prompt(&geocoder, provider.as_ref()).await;
}

fn print_banner() {
    println!("{}", "╔══════════════════════════════════════╗".cyan().bold());
    println!("{}", "║       Location Simulator             ║".cyan().bold());
    println!("{}", "╚══════════════════════════════════════╝".cyan().bold());
    println!();
}

/// Prompt helper that reads a line from stdin.
fn prompt_line(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        input.trim().to_string()
    } else {
        String::new()
    }
}

/// Confirm with user (default is Yes).
fn confirm_prompt(prompt: &str, auto_yes: bool) -> bool {
    if auto_yes {
        return true;
    }

    print!("{} [Y/n] > ", prompt);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
    } else {
        false
    }
}

/// Display location details nicely formatted.
fn display_location_card(location: &Location) {
    println!();
    println!("{}", "✓ Location found".green().bold());
    println!();
    println!("Location : {}", location.name.bold());
    println!("Address  : {}", location.formatted_address(11));
    println!();
    println!("Latitude : {:.6}", location.latitude);
    println!("Longitude: {:.6}", location.longitude);
    println!();
}

async fn handle_search_query(
    geocoder: &impl Geocoder,
    provider: &dyn LocationProvider,
    query: &str,
    auto_yes: bool,
) {
    println!("{}", "Searching location...".yellow());

    match geocoder.geocode(query).await {
        Ok(loc) => {
            display_location_card(&loc);
            if confirm_prompt("Set this as simulated location?", auto_yes) {
                apply_location(provider, &loc);
            } else {
                println!("{}", "Operation cancelled.".yellow());
            }
        }
        Err(err) => {
            eprintln!("{} {}", "✗ Geocoding failed:".red().bold(), err);
        }
    }
}

async fn handle_manual_coordinates(
    geocoder: &impl Geocoder,
    provider: &dyn LocationProvider,
    lat: f64,
    lon: f64,
    auto_yes: bool,
) {
    if let Err(e) = Location::validate_coordinates(lat, lon) {
        eprintln!("{} {}", "✗ Coordinate validation error:".red().bold(), e);
        std::process::exit(1);
    }

    println!("{}", "Resolving coordinates...".yellow());
    let location = geocoder
        .reverse_geocode(lat, lon)
        .await
        .unwrap_or_else(|_| {
            Location::new(
                format!("{:.5}, {:.5}", lat, lon),
                format!("Manual Coordinates: {:.5}, {:.5}", lat, lon),
                lat,
                lon,
            )
            .expect("Validated coordinates")
        });

    display_location_card(&location);
    if confirm_prompt("Set this as simulated location?", auto_yes) {
        apply_location(provider, &location);
    } else {
        println!("{}", "Operation cancelled.".yellow());
    }
}

fn apply_location(provider: &dyn LocationProvider, location: &Location) {
    match provider.set_location(location) {
        Ok(()) => {
            println!();
            println!("{}", "✓ Simulated location successfully applied!".green().bold());
            println!("  Provider: {}", provider.provider_name().cyan());
            let diag = provider.diagnostics();
            println!("  OS Mechanism: {}", diag.mechanism_name);
            println!();
            println!("{}", "Generated integration files:".bold());
            for f in &diag.exported_files {
                println!("  • {}", f.display().to_string().cyan());
            }
            if let Some(note) = diag.elevation_note {
                println!();
                println!("  {} {}", "Notice:".yellow().bold(), note);
            }
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Failed to set simulated location:".red().bold(), e);
        }
    }
}

fn show_current_location(provider: &dyn LocationProvider) {
    println!("{}", "══════════════════════════════════════".cyan());
    println!("{}", "       Simulated Location Status      ".bold());
    println!("{}", "══════════════════════════════════════".cyan());
    println!();

    match provider.get_location() {
        Ok(Some(loc)) => {
            println!("Status   : {}", "ACTIVE SIMULATION".green().bold());
            println!("Location : {}", loc.name.bold());
            println!("Address  : {}", loc.formatted_address(11));
            println!("Latitude : {:.6}", loc.latitude);
            println!("Longitude: {:.6}", loc.longitude);
        }
        Ok(None) => {
            println!("Status   : {}", "NO SIMULATED LOCATION CONFIGURED".yellow().bold());
            println!("Run `locsim \"<location>\"` or `locsim --interactive` to set one.");
        }
        Err(e) => {
            eprintln!("{} {}", "Error reading simulated location:".red().bold(), e);
        }
    }

    println!();
    let diag = provider.diagnostics();
    println!("{}", "OS & Platform Integration:".bold());
    println!("  Platform   : {}", diag.platform_name);
    println!("  Provider   : {}", provider.provider_name());
    println!("  Mechanism  : {}", diag.mechanism_name);
    println!(
        "  Elevated   : {}",
        if diag.is_elevated {
            "Yes (Administrator)".green()
        } else {
            "No (Standard user)".dimmed()
        }
    );
    println!("  Details    : {}", diag.os_mechanism_description);
    println!();
    println!("{}", "Artifacts & Exports:".bold());
    for path in &diag.exported_files {
        let status = if path.exists() {
            "[EXISTS]".green()
        } else {
            "[NOT PRESENT]".dimmed()
        };
        println!("  • {} {}", path.display(), status);
    }

    println!();
    print_limitations_overview(&diag);
}

fn clear_location(provider: &dyn LocationProvider) {
    match provider.clear_location() {
        Ok(()) => {
            println!("{}", "✓ Simulated location successfully cleared.".green().bold());
            println!("  Reset overrides and cleaned up local mock artifacts.");
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Failed to clear simulated location:".red().bold(), e);
        }
    }
}

fn print_limitations_overview(diag: &platform::ProviderDiagnostics) {
    println!("{}", "──────────────────────────────────────────────────────".dimmed());
    println!("{}", "Location Simulation Scope & Architecture:".bold());
    println!("{}", "──────────────────────────────────────────────────────".dimmed());
    println!(
        "1. {} Stored in persistent config ({}) and environment script. Compatible CLI tools, tests, and scripts read this immediately.",
        "CLI / Local Environment:".cyan().bold(),
        "current_location.json"
    );
    println!(
        "2. {} {}",
        "OS-Level Services:".cyan().bold(),
        diag.os_mechanism_description
    );
    println!(
        "3. {} {}",
        "Browser Geolocation:".cyan().bold(),
        diag.browser_integration_guide
    );
    println!(
        "4. {} Remote servers and websites determining location via incoming IP address (GeoIP) are unaffected. GPS simulation cannot alter network routing IP.",
        "IP-Based Geolocation:".yellow().bold()
    );
    println!(
        "5. {} Mobile apps, cell-tower triangulation, or standalone apps scanning local Wi-Fi BSSIDs independently bypass simulated coordinates unless system sensor drivers are overridden.",
        "Independent Apps:".yellow().bold()
    );
    println!("{}", "──────────────────────────────────────────────────────".dimmed());
}

async fn run_direct_prompt(geocoder: &impl Geocoder, provider: &dyn LocationProvider) {
    println!("Enter a location:");
    let input = prompt_line("> ");
    if input.is_empty() {
        println!("{}", "No location entered. Exiting.".yellow());
        return;
    }

    handle_search_query(geocoder, provider, &input, false).await;
}

async fn run_interactive_menu(geocoder: &impl Geocoder, provider: &dyn LocationProvider) {
    loop {
        println!();
        println!("{}", "Location Simulator Menu:".cyan().bold());
        println!("1. Search for a location");
        println!("2. Enter coordinates manually");
        println!("3. Show current simulated location");
        println!("4. Clear simulated location");
        println!("5. Exit");
        println!();

        let choice = prompt_line("Select an option [1-5] > ");
        match choice.as_str() {
            "1" => {
                let query = prompt_line("Enter search query > ");
                if !query.is_empty() {
                    handle_search_query(geocoder, provider, &query, false).await;
                }
            }
            "2" => {
                let lat_str = prompt_line("Enter latitude (-90.0 to 90.0) > ");
                let lon_str = prompt_line("Enter longitude (-180.0 to 180.0) > ");

                match (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                    (Ok(lat), Ok(lon)) => {
                        handle_manual_coordinates(geocoder, provider, lat, lon, false).await;
                    }
                    _ => {
                        eprintln!("{}", "Invalid number format for latitude or longitude.".red());
                    }
                }
            }
            "3" => {
                show_current_location(provider);
            }
            "4" => {
                clear_location(provider);
            }
            "5" | "q" | "exit" => {
                println!("{}", "Goodbye!".cyan());
                break;
            }
            _ => {
                println!("{}", "Invalid option. Please enter 1, 2, 3, 4, or 5.".red());
            }
        }
    }
}

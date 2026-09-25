use clap::{CommandFactory, Parser};
use clap_complete::generate;
use colored::*;
use std::io::{self, Write};

mod bookmarks;
mod cli;
mod config;
mod geocoder;
mod location;
mod platform;
mod real_location;

use bookmarks::BookmarkManager;
use cli::{Cli, Commands};
use config::Config;
use geocoder::{create_geocoder, CachedGeocoder, Geocoder};
use location::Location;
use platform::{get_platform_provider, LocationProvider};

#[derive(Debug, Clone, Copy, Default)]
struct TelemetryArgs {
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub speed: Option<f64>,
    pub heading: Option<f64>,
}

fn attach_telemetry(mut location: Location, telemetry: TelemetryArgs) -> Location {
    if telemetry.altitude.is_some() {
        location.altitude = telemetry.altitude;
    }
    if telemetry.accuracy.is_some() {
        location.accuracy = telemetry.accuracy;
    }
    if telemetry.speed.is_some() {
        location.speed = telemetry.speed;
    }
    if telemetry.heading.is_some() {
        location.heading = telemetry.heading;
    }
    location
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mut config = Config::load();

    if let Some(ref custom_url) = cli.geocoder_url {
        config.geocoder_url = custom_url.clone();
    }
    if let Some(provider) = cli.provider {
        config.provider = provider;
    }
    if let Some(ref key) = cli.api_key {
        config.api_key = Some(key.clone());
    }

    let telemetry = TelemetryArgs {
        altitude: cli.altitude,
        accuracy: cli.accuracy,
        speed: cli.speed,
        heading: cli.heading,
    };

    if let Err(e) = Location::validate_telemetry(cli.altitude, cli.accuracy, cli.speed, cli.heading) {
        eprintln!("{} {}", "✗ Telemetry validation error:".red().bold(), e);
        std::process::exit(1);
    }

    let raw_geocoder = match create_geocoder(config.provider, config.api_key.clone(), &config) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{} {}", "✗ Geocoder configuration error:".red().bold(), e);
            std::process::exit(1);
        }
    };
    let geocoder = CachedGeocoder::new(raw_geocoder, &config);
    let provider = get_platform_provider();
    let bookmark_mgr = BookmarkManager::new();
    let android_device = cli.android_device.as_deref();

    // 1. Subcommands (locsim save, use, list, delete, reset, completions)
    if let Some(ref cmd) = cli.command {
        match cmd {
            Commands::Save { name, query } => {
                handle_save_bookmark(&bookmark_mgr, &geocoder, provider.as_ref(), name, query.as_deref(), telemetry, android_device, cli.yes).await;
                return;
            }
            Commands::Use { name } => {
                handle_use_bookmark(&bookmark_mgr, provider.as_ref(), name, telemetry, android_device);
                return;
            }
            Commands::List => {
                handle_list_bookmarks(&bookmark_mgr);
                return;
            }
            Commands::Delete { name } => {
                handle_delete_bookmark(&bookmark_mgr, name);
                return;
            }
            Commands::Reset => {
                handle_reset_to_real(provider.as_ref()).await;
                return;
            }
            Commands::Completions { shell } => {
                handle_completions(*shell);
                return;
            }
        }
    }

    // 2. Flag equivalents
    if let Some(shell) = cli.completions {
        handle_completions(shell);
        return;
    }

    if cli.reset {
        handle_reset_to_real(provider.as_ref()).await;
        return;
    }

    if cli.bookmarks {
        handle_list_bookmarks(&bookmark_mgr);
        return;
    }

    if let Some(ref name) = cli.delete_bookmark {
        handle_delete_bookmark(&bookmark_mgr, name);
        return;
    }

    if let Some(ref name) = cli.r#use {
        handle_use_bookmark(&bookmark_mgr, provider.as_ref(), name, telemetry, android_device);
        return;
    }

    if let Some(ref name) = cli.save {
        handle_save_bookmark(&bookmark_mgr, &geocoder, provider.as_ref(), name, cli.query.as_deref(), telemetry, android_device, cli.yes).await;
        return;
    }

    // 3. locsim --show
    if cli.show {
        show_current_location(provider.as_ref(), &config);
        return;
    }

    // 4. locsim --clear
    if cli.clear {
        clear_location(provider.as_ref());
        return;
    }

    // 5. locsim --interactive / -i
    if cli.interactive {
        run_interactive_menu(&geocoder, provider.as_ref(), &bookmark_mgr, android_device, &config).await;
        return;
    }


    // 6. locsim --reverse <LAT> <LON> / -r <LAT> <LON>
    if let Some(ref coords) = cli.reverse {
        if coords.len() == 2 {
            handle_reverse_query(&geocoder, provider.as_ref(), coords[0], coords[1], telemetry, android_device, cli.yes).await;
            return;
        } else {
            eprintln!(
                "{}",
                "Error: --reverse requires exactly 2 arguments: <LATITUDE> <LONGITUDE>".red().bold()
            );
            std::process::exit(1);
        }
    }

    // 7. locsim --lat <LAT> --lon <LON>
    if cli.lat.is_some() || cli.lon.is_some() {
        match (cli.lat, cli.lon) {
            (Some(lat), Some(lon)) => {
                handle_manual_coordinates(&geocoder, provider.as_ref(), lat, lon, telemetry, android_device, cli.yes).await;
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

    // 8. locsim "query"
    if let Some(ref query) = cli.query {
        handle_search_query(&geocoder, provider.as_ref(), query, telemetry, android_device, cli.yes).await;
        return;
    }

    // 9. locsim (default no args) -> Quick Interactive Search Flow
    print_banner();
    run_direct_prompt(&geocoder, provider.as_ref(), android_device).await;
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
    if let Some(alt) = location.altitude {
        println!("Altitude : {:.2} m", alt);
    }
    if let Some(acc) = location.accuracy {
        println!("Accuracy : {:.2} m", acc);
    }
    if let Some(spd) = location.speed {
        println!("Speed    : {:.2} m/s", spd);
    }
    if let Some(hdg) = location.heading {
        println!("Heading  : {:.2}°", hdg);
    }
    println!();
}

async fn handle_search_query(
    geocoder: &impl Geocoder,
    provider: &dyn LocationProvider,
    query: &str,
    telemetry: TelemetryArgs,
    android_device: Option<&str>,
    auto_yes: bool,
) {
    println!("{}", "Searching location...".yellow());

    match geocoder.geocode(query).await {
        Ok(loc) => {
            let loc = attach_telemetry(loc, telemetry);
            display_location_card(&loc);
            if confirm_prompt("Set this as simulated location?", auto_yes) {
                apply_location(provider, &loc, android_device);
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
    telemetry: TelemetryArgs,
    android_device: Option<&str>,
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

    let location = attach_telemetry(location, telemetry);

    display_location_card(&location);
    if confirm_prompt("Set this as simulated location?", auto_yes) {
        apply_location(provider, &location, android_device);
    } else {
        println!("{}", "Operation cancelled.".yellow());
    }
}

async fn handle_reverse_query(
    geocoder: &impl Geocoder,
    provider: &dyn LocationProvider,
    lat: f64,
    lon: f64,
    telemetry: TelemetryArgs,
    android_device: Option<&str>,
    auto_yes: bool,
) {
    if let Err(e) = Location::validate_coordinates(lat, lon) {
        eprintln!("{} {}", "✗ Coordinate validation error:".red().bold(), e);
        std::process::exit(1);
    }

    println!("{}", "Reverse geocoding coordinates...".yellow());
    match geocoder.reverse_geocode(lat, lon).await {
        Ok(loc) => {
            let loc = attach_telemetry(loc, telemetry);
            display_location_card(&loc);
            if auto_yes {
                apply_location(provider, &loc, android_device);
            }
        }
        Err(err) => {
            eprintln!("{} {}", "✗ Reverse geocoding failed:".red().bold(), err);
            std::process::exit(1);
        }
    }
}

fn apply_location(provider: &dyn LocationProvider, location: &Location, android_device: Option<&str>) {
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

    // Android emulator integration
    match platform::android::AndroidBridge::send_geo_fix(android_device, location) {
        Ok(devices) if !devices.is_empty() => {
            println!();
            println!("{}", "Mobile Emulator Integration:".bold());
            for dev in devices {
                println!("  • Applied geo fix to Android emulator: {}", dev.cyan());
            }
        }
        _ => {}
    }
}

fn show_current_location(provider: &dyn LocationProvider, config: &Config) {
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
            if let Some(alt) = loc.altitude {
                println!("Altitude : {:.2} m", alt);
            }
            if let Some(acc) = loc.accuracy {
                println!("Accuracy : {:.2} m", acc);
            }
            if let Some(spd) = loc.speed {
                println!("Speed    : {:.2} m/s", spd);
            }
            if let Some(hdg) = loc.heading {
                println!("Heading  : {:.2}°", hdg);
            }
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
    println!("  Geocoder   : {}", config.provider.to_string().cyan());
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
    let android_status = platform::android::AndroidBridge::status();
    println!("{}", "Mobile Emulator Status:".bold());
    if android_status.adb_available {
        println!(
            "  ADB        : {} ({})",
            "Available".green(),
            android_status.adb_version.unwrap_or_else(|| "Detected".to_string())
        );
        if android_status.running_emulators.is_empty() {
            println!("  Emulators  : None connected (run `emulator @<avd>` to auto-sync)");
        } else {
            for emu in &android_status.running_emulators {
                println!("  Emulator   : {} {}", emu.cyan(), "[ONLINE]".green());
            }
        }
    } else {
        println!("  ADB        : {}", "Not detected in PATH".dimmed());
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
        "1. {} Stored in persistent config (current_location.json) and environment script. Compatible CLI tools, tests, and scripts read this immediately.",
        "CLI / Local Environment:".cyan().bold()
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

#[allow(clippy::too_many_arguments)]
async fn handle_save_bookmark(
    bookmark_mgr: &BookmarkManager,
    geocoder: &impl Geocoder,
    provider: &dyn LocationProvider,
    name: &str,
    query: Option<&str>,
    telemetry: TelemetryArgs,
    android_device: Option<&str>,
    auto_yes: bool,
) {

    let location = if let Some(q) = query {
        println!("{}", format!("Searching '{}' to save as bookmark '{}'...", q, name).yellow());
        match geocoder.geocode(q).await {
            Ok(loc) => attach_telemetry(loc, telemetry),
            Err(e) => {
                eprintln!("{} {}", "✗ Geocoding failed:".red().bold(), e);
                std::process::exit(1);
            }
        }
    } else {
        match provider.get_location() {
            Ok(Some(loc)) => attach_telemetry(loc, telemetry),
            _ => {
                eprintln!(
                    "{}",
                    "✗ No active simulated location found to save. Specify a location: `locsim save <name> <location>`".red().bold()
                );
                std::process::exit(1);
            }
        }
    };

    match bookmark_mgr.save(name, &location) {
        Ok(()) => {
            println!("{}", format!("✓ Saved bookmark '{}' -> {}", name.cyan().bold(), location.name.bold()).green());
            println!("  Coords: {:.6}, {:.6}", location.latitude, location.longitude);
            println!("  Use anytime via: `locsim use {}`", name);

            if query.is_some() && auto_yes {
                apply_location(provider, &location, android_device);
            }
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Failed to save bookmark:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn handle_use_bookmark(
    bookmark_mgr: &BookmarkManager,
    provider: &dyn LocationProvider,
    name: &str,
    telemetry: TelemetryArgs,
    android_device: Option<&str>,
) {
    match bookmark_mgr.get(name) {
        Ok(Some(mut loc)) => {
            loc = attach_telemetry(loc, telemetry);
            println!("{}", format!("Applying saved bookmark '{}':", name.cyan().bold()).green());
            display_location_card(&loc);
            apply_location(provider, &loc, android_device);
        }
        Ok(None) => {
            eprintln!(
                "{} Bookmark '{}' not found. Run 'locsim list' to view available bookmarks.",
                "✗".red().bold(),
                name.yellow()
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Failed to read bookmarks:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn handle_list_bookmarks(bookmark_mgr: &BookmarkManager) {
    match bookmark_mgr.list() {
        Ok(list) => {
            if list.is_empty() {
                println!("{}", "No saved bookmarks yet.".yellow());
                println!("Save one using: `locsim save <name> [location]`");
                return;
            }

            println!("{}", "╔══════════════════════════════════════════════════════════════╗".cyan().bold());
            println!("{}", "║                     Saved Location Bookmarks                 ║".cyan().bold());
            println!("{}", "╚══════════════════════════════════════════════════════════════╝".cyan().bold());
            println!();

            for (name, loc) in &list {
                println!("• {}", name.cyan().bold());
                println!("  Location : {}", loc.name);
                println!("  Address  : {}", loc.formatted_address(13));
                println!("  Coords   : {:.6}, {:.6}", loc.latitude, loc.longitude);
                let mut tele = Vec::new();
                if let Some(alt) = loc.altitude {
                    tele.push(format!("Alt: {:.1}m", alt));
                }
                if let Some(acc) = loc.accuracy {
                    tele.push(format!("Acc: {:.1}m", acc));
                }
                if let Some(spd) = loc.speed {
                    tele.push(format!("Speed: {:.1}m/s", spd));
                }
                if let Some(hdg) = loc.heading {
                    tele.push(format!("Heading: {:.0}°", hdg));
                }
                if !tele.is_empty() {
                    println!("  Telemetry: {}", tele.join(" | ").dimmed());
                }
                println!();
            }

            println!("Total: {} bookmark(s) saved.", list.len());
            println!("Apply anytime using: `locsim use <name>`");
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Failed to load bookmarks:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn handle_delete_bookmark(bookmark_mgr: &BookmarkManager, name: &str) {
    match bookmark_mgr.delete(name) {
        Ok(true) => {
            println!("{}", format!("✓ Deleted bookmark '{}'.", name.cyan()).green().bold());
        }
        Ok(false) => {
            eprintln!(
                "{} Bookmark '{}' not found. Run 'locsim list' to view available bookmarks.",
                "✗".red().bold(),
                name.yellow()
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{} {}", "✗ Failed to delete bookmark:".red().bold(), e);
            std::process::exit(1);
        }
    }
}

async fn handle_reset_to_real(provider: &dyn LocationProvider) {
    clear_location(provider);

    println!();
    println!("{}", "Detecting real physical location via IP Geolocation...".yellow());
    match real_location::fetch_real_location().await {
        Ok(info) => {
            println!();
            println!("{}", "══════════════════════════════════════".cyan());
            println!("{}", "       Real Physical Location (IP)    ".bold());
            println!("{}", "══════════════════════════════════════".cyan());
            println!();
            if let Some(ip) = info.ip {
                println!("Public IP  : {}", ip.bold());
            }
            let city = info.city.unwrap_or_else(|| "Unknown".to_string());
            let region = info.region.unwrap_or_default();
            let country = info.country.unwrap_or_default();
            println!("Location   : {}, {}, {}", city, region, country);
            if let (Some(lat), Some(lon)) = (info.lat, info.lon) {
                println!("Coordinates: {:.6}, {:.6}", lat, lon);
            }
            if let Some(isp) = info.isp {
                println!("ISP / Org  : {}", isp.dimmed());
            }
            println!();
            println!("{}", "Device is now reporting its genuine network location.".green());
        }
        Err(e) => {
            println!("{}", format!("Notice: Could not query real location (offline or service unreachable): {}", e).yellow());
        }
    }
}

async fn run_direct_prompt(geocoder: &impl Geocoder, provider: &dyn LocationProvider, android_device: Option<&str>) {
    println!("Enter a location:");
    let input = prompt_line("> ");
    if input.is_empty() {
        println!("{}", "No location entered. Exiting.".yellow());
        return;
    }

    handle_search_query(geocoder, provider, &input, TelemetryArgs::default(), android_device, false).await;
}

async fn run_interactive_menu(
    geocoder: &impl Geocoder,
    provider: &dyn LocationProvider,
    bookmark_mgr: &BookmarkManager,
    android_device: Option<&str>,
    config: &Config,
) {
    loop {
        println!();
        println!("{}", "Location Simulator Menu:".cyan().bold());
        println!("1. Search for a location");
        println!("2. Enter coordinates manually");
        println!("3. Show current simulated location");
        println!("4. Clear simulated location");
        println!("5. Saved location bookmarks (List / Use / Save)");
        println!("6. Reset to real physical location");
        println!("7. Exit");
        println!();

        let choice = prompt_line("Select an option [1-7] > ");
        match choice.as_str() {
            "1" => {
                let query = prompt_line("Enter search query > ");
                if !query.is_empty() {
                    handle_search_query(geocoder, provider, &query, TelemetryArgs::default(), android_device, false).await;
                }
            }
            "2" => {
                let lat_str = prompt_line("Enter latitude (-90.0 to 90.0) > ");
                let lon_str = prompt_line("Enter longitude (-180.0 to 180.0) > ");

                match (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                    (Ok(lat), Ok(lon)) => {
                        handle_manual_coordinates(geocoder, provider, lat, lon, TelemetryArgs::default(), android_device, false).await;
                    }
                    _ => {
                        eprintln!("{}", "Invalid number format for latitude or longitude.".red());
                    }
                }
            }
            "3" => {
                show_current_location(provider, config);
            }

            "4" => {
                clear_location(provider);
            }
            "5" => {
                println!();
                println!("Bookmarks: [L]ist, [U]se, [S]ave, [D]elete");
                let bm_choice = prompt_line("Action [L/u/s/d] > ").to_lowercase();
                match bm_choice.as_str() {
                    "u" | "use" => {
                        let name = prompt_line("Bookmark name to use > ");
                        if !name.is_empty() {
                            handle_use_bookmark(bookmark_mgr, provider, &name, TelemetryArgs::default(), android_device);
                        }
                    }
                    "s" | "save" => {
                        let name = prompt_line("Bookmark name > ");
                        let loc_query = prompt_line("Location query (leave blank to save current) > ");
                        let q = if loc_query.is_empty() { None } else { Some(loc_query.as_str()) };
                        if !name.is_empty() {
                            handle_save_bookmark(bookmark_mgr, geocoder, provider, &name, q, TelemetryArgs::default(), android_device, false).await;
                        }
                    }
                    "d" | "delete" => {
                        let name = prompt_line("Bookmark name to delete > ");
                        if !name.is_empty() {
                            handle_delete_bookmark(bookmark_mgr, &name);
                        }
                    }
                    _ => {
                        handle_list_bookmarks(bookmark_mgr);
                    }
                }
            }
            "6" => {
                handle_reset_to_real(provider).await;
            }
            "7" | "q" | "exit" => {
                println!("{}", "Goodbye!".cyan());
                break;
            }
            _ => {
                println!("{}", "Invalid option. Please enter 1 to 7.".red());
            }
        }
    }
}

fn handle_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "locsim", &mut io::stdout());
}

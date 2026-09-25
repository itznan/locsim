use assert_cmd::Command;
use predicates::prelude::*;
use std::sync::Mutex;

static CLI_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Universal CLI-based location simulator"))
        .stdout(predicate::str::contains("--lat"))
        .stdout(predicate::str::contains("--lon"))
        .stdout(predicate::str::contains("--show"))
        .stdout(predicate::str::contains("--clear"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("locsim 0.1.0"));
}

#[test]
fn test_cli_missing_longitude_errors() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--lat", "37.7749"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Both --lat and --lon must be provided together"));
}

#[test]
fn test_cli_missing_latitude_errors() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--lon", "-122.4194"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Both --lat and --lon must be provided together"));
}

#[test]
fn test_cli_invalid_coordinate_bounds() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--lat", "120.0", "--lon", "0.0", "-y"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("out of valid range"));
}

#[test]
fn test_cli_show() {
    let _lock = CLI_LOCK.lock().unwrap();
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("--show");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Simulated Location Status"));
}

#[test]
fn test_cli_clear() {
    let _lock = CLI_LOCK.lock().unwrap();
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("--clear");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Simulated location successfully cleared"));
}

#[test]
fn test_cli_help_includes_telemetry_and_reverse() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--reverse"))
        .stdout(predicate::str::contains("--altitude"))
        .stdout(predicate::str::contains("--accuracy"))
        .stdout(predicate::str::contains("--speed"))
        .stdout(predicate::str::contains("--heading"));
}

#[test]
fn test_cli_reverse_invalid_coordinate_bounds() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--reverse", "95.0", "0.0"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("out of valid range"));
}

#[test]
fn test_cli_telemetry_validation_negative_accuracy() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--lat", "10.0", "--lon", "20.0", "--accuracy", "-5.0"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Telemetry validation error"))
        .stderr(predicate::str::contains("Accuracy must be a non-negative finite number"));
}

#[test]
fn test_cli_telemetry_validation_invalid_heading() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--lat", "10.0", "--lon", "20.0", "--heading", "400.0"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Telemetry validation error"))
        .stderr(predicate::str::contains("Heading must be between 0.0 and 360.0 degrees"));
}

#[test]
fn test_cli_set_manual_with_telemetry_and_show() {
    let _lock = CLI_LOCK.lock().unwrap();
    let mut set_cmd = Command::cargo_bin("locsim").unwrap();
    set_cmd.args([
        "--lat", "37.7749",
        "--lon", "-122.4194",
        "--altitude", "12.5",
        "--accuracy", "8.0",
        "--speed", "4.2",
        "--heading", "180.0",
        "-y",
    ]);
    set_cmd.assert()
        .success()
        .stdout(predicate::str::contains("Simulated location successfully applied"));

    let mut show_cmd = Command::cargo_bin("locsim").unwrap();
    show_cmd.arg("--show");
    show_cmd.assert()
        .success()
        .stdout(predicate::str::contains("37.774900"))
        .stdout(predicate::str::contains("-122.419400"))
        .stdout(predicate::str::contains("Altitude : 12.50 m"))
        .stdout(predicate::str::contains("Accuracy : 8.00 m"))
        .stdout(predicate::str::contains("Speed    : 4.20 m/s"))
        .stdout(predicate::str::contains("Heading  : 180.00°"));

    // Cleanup
    let mut clear_cmd = Command::cargo_bin("locsim").unwrap();
    clear_cmd.arg("--clear");
    clear_cmd.assert().success();
}

#[test]
fn test_cli_bookmark_save_list_use_delete() {
    let _lock = CLI_LOCK.lock().unwrap();
    // 1. Set a manual location
    let mut set_cmd = Command::cargo_bin("locsim").unwrap();
    set_cmd.args(["--lat", "34.0522", "--lon", "-118.2437", "-y"]);
    set_cmd.assert().success();

    // 2. Save current location as bookmark 'la'
    let mut save_cmd = Command::cargo_bin("locsim").unwrap();
    save_cmd.args(["save", "la_test_spot"]);
    save_cmd.assert()
        .success()
        .stdout(predicate::str::contains("Saved bookmark 'la_test_spot'"));

    // 3. List bookmarks
    let mut list_cmd = Command::cargo_bin("locsim").unwrap();
    list_cmd.arg("list");
    list_cmd.assert()
        .success()
        .stdout(predicate::str::contains("la_test_spot"))
        .stdout(predicate::str::contains("34.052200"));

    // 4. Clear simulated location
    let mut clear_cmd = Command::cargo_bin("locsim").unwrap();
    clear_cmd.arg("--clear");
    clear_cmd.assert().success();

    // 5. Use bookmark 'la_test_spot'
    let mut use_cmd = Command::cargo_bin("locsim").unwrap();
    use_cmd.args(["use", "la_test_spot"]);
    use_cmd.assert()
        .success()
        .stdout(predicate::str::contains("Applying saved bookmark 'la_test_spot'"))
        .stdout(predicate::str::contains("34.052200"));

    // 6. Delete bookmark 'la_test_spot'
    let mut del_cmd = Command::cargo_bin("locsim").unwrap();
    del_cmd.args(["delete", "la_test_spot"]);
    del_cmd.assert()
        .success()
        .stdout(predicate::str::contains("Deleted bookmark 'la_test_spot'"));

    // 7. Cleanup simulated location
    let mut clean_cmd = Command::cargo_bin("locsim").unwrap();
    clean_cmd.arg("--clear");
    clean_cmd.assert().success();
}

#[test]
fn test_cli_use_nonexistent_bookmark() {
    let mut use_cmd = Command::cargo_bin("locsim").unwrap();
    use_cmd.args(["use", "definitely_nonexistent_bookmark_123"]);
    use_cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Bookmark 'definitely_nonexistent_bookmark_123' not found"));
}

#[test]
fn test_cli_reset() {
    let _lock = CLI_LOCK.lock().unwrap();
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("reset");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Simulated location successfully cleared"));
}

#[test]
fn test_cli_completions_subcommand_powershell() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["completions", "powershell"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Register-ArgumentCompleter"))
        .stdout(predicate::str::contains("locsim"));
}

#[test]
fn test_cli_completions_subcommand_bash() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["completions", "bash"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_locsim()"))
        .stdout(predicate::str::contains("complete -F _locsim"));
}

#[test]
fn test_cli_completions_flag_zsh() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--completions", "zsh"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("#compdef locsim"));
}

#[test]
fn test_cli_provider_flag_in_help() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--provider"))
        .stdout(predicate::str::contains("--api-key"));
}

#[test]
fn test_cli_provider_missing_api_key_fails() {
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--provider", "mapbox", "Eiffel Tower", "-y"]);
    cmd.assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("API key required for provider 'Mapbox'"));
}

#[test]
fn test_cli_show_displays_geocoder_provider() {
    let _lock = CLI_LOCK.lock().unwrap();
    let mut cmd = Command::cargo_bin("locsim").unwrap();
    cmd.args(["--show", "--provider", "nominatim"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Geocoder   : nominatim"));
}




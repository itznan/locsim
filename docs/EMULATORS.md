# Mobile Emulator Testing with `locsim`

`locsim` provides native bridges to both **Android Virtual Devices (AVD)** and **Apple iOS/watchOS Simulators**, enabling automated location spoofing without manually opening emulator settings.

---

## 1. Android Emulator (ADB Integration)

`locsim` communicates directly with Android emulators over the Android Debug Bridge (`adb`).

### Automatic Discovery
When you execute any `locsim` command:
1. `locsim` inspects `PATH`, `$ANDROID_HOME/platform-tools`, and standard SDK directories (`%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe` on Windows, `~/Library/Android/sdk/platform-tools/adb` on macOS).
2. It queries connected emulators via `adb devices`.
3. If an emulator is online (e.g. `emulator-5554`), `locsim` dispatches the command:
   ```bash
   adb -s emulator-5554 emu geo fix <lon> <lat> [alt]
   ```
   > **Note on coordinate order:** Android's `emu geo fix` command takes **longitude** first, then **latitude** and optional altitude. `locsim` automatically handles this ordering for you.

4. If `emu geo fix` fails or is not supported by a custom ROM, `locsim` automatically executes the fallback:
   ```bash
   adb -s emulator-5554 shell cmd location set-location <lat> <lon>
   ```

### Targeting Specific Emulators
If you have multiple emulators running (e.g., tablet and phone), target one by serial:

```bash
# Check running emulators:
locsim --show

# Target a specific device:
locsim "Tokyo, Japan" --android-device emulator-5554 -y
```

### Verifying in Android App
Once `locsim` sets the location:
1. Open Google Maps or your app inside the Android emulator.
2. The blue location dot will instantly snap to your simulated coordinates.
3. Telemetry fields (altitude, accuracy) are registered by `FusedLocationProviderClient` and Android's `LocationManager`.

---

## 2. iOS & iPadOS Simulator (macOS Xcode Integration)

On macOS, `locsim` integrates with Apple's `xcrun simctl` command-line utility.

### Automatic Injection
When run on macOS with a booted iOS or watchOS simulator:
```bash
xcrun simctl location booted set <lat> <lon>
```

### GPX Playback for Xcode Instruments
`locsim` automatically updates `simulated_location.gpx`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="locsim 0.1.0">
  <wpt lat="37.774900" lon="-122.419400">
    <ele>12.50</ele>
    <time>2026-09-25T12:00:00Z</time>
    <name>San Francisco</name>
  </wpt>
</gpx>
```

In Xcode:
1. Run your iOS app in the simulator.
2. In Xcode's debug bar, click the **Location Arrow** icon.
3. Select **Add GPX File to Project...** and point to `simulated_location.gpx`.
4. Your simulator will immediately report the simulated location.

---

## 3. End-to-End Mobile Testing Workflow

```bash
# 1. Start your Android or iOS simulator
emulator @Pixel_7_Pro_API_34 &

# 2. Set test position
locsim "Golden Gate Bridge" -y

# 3. Simulate high-speed vehicle movement with telemetry
locsim --lat 37.8199 --lon -122.4783 --speed 25.0 --heading 350.0 --altitude 67.0 -y

# 4. Check simulator status
locsim --show

# 5. Reset to real physical location when finished
locsim reset
```

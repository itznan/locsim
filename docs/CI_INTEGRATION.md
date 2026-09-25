# Continuous Integration & Automated Testing with `locsim`

`locsim` is designed to be deterministic and scriptable, making it ideal for CI/CD test suites (GitHub Actions, GitLab CI, Jenkins) where location-aware services must be verified reliably.

---

## 1. Browser Test Automation (Playwright & Puppeteer)

`locsim` writes Chrome DevTools Protocol (`cdp_geolocation.json`) on every run. You can load this file directly in your headless browser test runners.

### Playwright (TypeScript)

```typescript
import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

test('verify location-dependent checkout pricing', async ({ browser }) => {
  // Read locsim's exported CDP geolocation JSON
  const cdpPath = path.join(process.env.APPDATA || process.env.HOME || '', '.locsim', 'cdp_geolocation.json');
  const geo = JSON.parse(fs.readFileSync(cdpPath, 'utf-8'));

  const context = await browser.newContext({
    geolocation: {
      latitude: geo.latitude,
      longitude: geo.longitude,
      accuracy: geo.accuracy || 10,
    },
    permissions: ['geolocation'],
  });

  const page = await context.newPage();
  await page.goto('https://my-location-store.com');

  // Verify your app detects the simulated city
  await expect(page.locator('#detected-city')).toHaveText('San Francisco');
});
```

### Puppeteer (JavaScript / Node.js)

```javascript
const puppeteer = require('puppeteer');
const fs = require('fs');

(async () => {
  const browser = await puppeteer.launch();
  const page = await browser.newPage();

  // Load locsim CDP payload
  const cdp = JSON.parse(fs.readFileSync('.locsim/cdp_geolocation.json', 'utf8'));

  // Override geolocation via CDP
  const client = await page.target().createCDPSession();
  await client.send('Emulation.setGeolocationOverride', {
    latitude: cdp.latitude,
    longitude: cdp.longitude,
    accuracy: cdp.accuracy || 1,
  });

  await page.goto('https://my-app.com/nearby');
  // ... run your assertions ...
  await browser.close();
})();
```

---

## 2. Python Test Suites (`pytest`)

```python
import json
import os
import subprocess
import pytest

@pytest.fixture(autouse=True)
def setup_simulated_location():
    # Set simulated location to London via locsim
    subprocess.run(["locsim", "London, UK", "-y"], check=True)
    yield
    # Reset back to real location
    subprocess.run(["locsim", "reset"], check=True)

def test_currency_conversion():
    # Read location JSON exported by locsim
    config_dir = os.environ.get("LOCSIM_DATA_DIR", os.path.expanduser("~/.config/locsim"))
    loc_file = os.path.join(config_dir, "current_location.json")
    
    with open(loc_file, "r") as f:
        location = json.load(f)

    assert location["name"] == "London"
    assert round(location["latitude"], 2) == 51.51
```

---

## 3. GitHub Actions CI Workflow

Here is a complete workflow demonstrating `locsim` running in GitHub Actions:

```yaml
name: Location-Aware Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - name: Checkout Code
        uses: actions/checkout@v4

      - name: Install Rust Toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache Cargo Dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install locsim
        run: cargo install --path .

      - name: Verify locsim Installation
        run: locsim --version

      - name: Simulate Tokyo Location for Test Suite
        run: |
          locsim "Tokyo Station, Japan" --altitude 20.0 --speed 0.0 -y
          locsim --show

      - name: Run Test Suite
        run: |
          # Source generated environment variables
          source ~/.config/locsim/locsim_env.sh
          echo "Testing at coordinates: $LOCSIM_LATITUDE, $LOCSIM_LONGITUDE"
          npm test
```

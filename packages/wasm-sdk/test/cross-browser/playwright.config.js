/**
 * Cross-Platform Browser Testing Configuration
 * Enhanced Playwright config for comprehensive browser compatibility testing
 */

const { defineConfig, devices } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './tests',
  
  // Run tests in parallel across different browsers
  fullyParallel: true,
  
  // Fail build on CI if you accidentally left test.only
  forbidOnly: !!process.env.CI,
  
  // Retry on CI only
  retries: process.env.CI ? 2 : 1,
  
  // Opt out of parallel tests on CI for more stable results
  workers: process.env.CI ? 1 : 3,
  
  // Reporter configuration
  reporter: [
    ['html', { outputFolder: 'cross-browser-report' }],
    ['json', { outputFile: 'cross-browser-results.json' }],
    ['list']
  ],

  // Global test settings
  use: {
    // Base URL for tests
    baseURL: 'http://localhost:8888',
    
    // Collect trace when retrying the failed test
    trace: 'on-first-retry',
    
    // Record video on failure
    video: 'retain-on-failure',
    
    // Take screenshot on failure
    screenshot: 'only-on-failure',
    
    // Extended timeout for WASM operations
    actionTimeout: 30000,
    navigationTimeout: 60000
  },

  // Configure projects for different browsers and devices
  projects: [
    // Desktop Browsers - Latest Versions
    {
      name: 'chromium-latest',
      use: { ...devices['Desktop Chrome'] }
    },
    {
      name: 'firefox-latest',
      use: { ...devices['Desktop Firefox'] }
    },
    {
      name: 'webkit-latest',
      use: { ...devices['Desktop Safari'] }
    },
    {
      name: 'edge-latest',
      use: { 
        ...devices['Desktop Edge'],
        channel: 'msedge'
      }
    },

    // Desktop Browsers - Minimum Supported Versions
    {
      name: 'chrome-80',
      use: {
        ...devices['Desktop Chrome'],
        channel: 'chrome',
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.149 Safari/537.36'
      }
    },
    {
      name: 'firefox-75',
      use: {
        ...devices['Desktop Firefox'],
        userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:75.0) Gecko/20100101 Firefox/75.0'
      }
    },

    // Mobile Devices - High-end
    {
      name: 'mobile-chrome-android',
      use: { ...devices['Pixel 5'] }
    },
    {
      name: 'mobile-safari-ios',
      use: { ...devices['iPhone 12'] }
    },
    {
      name: 'tablet-ipad',
      use: { ...devices['iPad Pro'] }
    },

    // Mobile Devices - Low-end (memory constraints testing)
    {
      name: 'mobile-low-end',
      use: { 
        ...devices['Galaxy S9+'],
        // Simulate lower memory device
        contextOptions: {
          deviceScaleFactor: 1,
          isMobile: true,
          hasTouch: true
        }
      }
    },

    // Specific Browser Features Testing
    {
      name: 'chromium-webassembly',
      use: {
        ...devices['Desktop Chrome'],
        // Test specific WASM features
        contextOptions: {
          permissions: ['clipboard-read', 'clipboard-write']
        }
      }
    }
  ],

  // Web server configuration
  webServer: {
    command: 'python3 -m http.server 8888',
    url: 'http://localhost:8888',
    cwd: '../../../', // Run from wasm-sdk parent directory
    reuseExistingServer: !process.env.CI,
    timeout: 120 * 1000
  },

  // Global test timeout
  timeout: 5 * 60 * 1000, // 5 minutes for WASM operations

  // Expect timeout
  expect: {
    timeout: 30000 // 30 seconds for assertions
  }
});
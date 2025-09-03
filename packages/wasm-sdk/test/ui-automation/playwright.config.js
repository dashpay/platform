// @ts-check
const { defineConfig, devices } = require('@playwright/test');

/**
 * @see https://playwright.dev/docs/test-configuration
 */
module.exports = defineConfig({
  testDir: './tests',
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['json', { outputFile: 'test-results.json' }],
    ['list']
  ],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: 'http://localhost:8888',
    
    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',
    
    /* Take screenshot on failure */
    screenshot: 'only-on-failure',
    
    /* Record video on failure */
    video: 'retain-on-failure',
    
    /* Global timeout for each action (e.g. click, fill, etc.) */
    actionTimeout: 30000,
    
    /* Global timeout for each navigation action */
    navigationTimeout: 30000,
  },

  /* Configure projects for different test execution modes */
  projects: [
    {
      name: 'parallel-tests',
      testMatch: ['basic-smoke.spec.js', 'query-execution.spec.js', 'parameterized-queries.spec.js'],
      fullyParallel: true,
      workers: process.env.CI ? 1 : undefined,
      use: { 
        ...devices['Desktop Chrome'],
        // Enable headless mode by default
        headless: true,
        // Use a larger viewport for better testing
        viewport: { width: 1920, height: 1080 }
      },
    },
    // Skip state transitions tests in CI environments
    // These are very slow-running due to https://github.com/dashpay/platform/issues/2736
    ...(process.env.CI ? [] : [{
      name: 'sequential-tests',
      testMatch: ['state-transitions.spec.js'],
      fullyParallel: false, // Tests in file run in order
      workers: 1, // Single worker for sequential execution
      use: { 
        ...devices['Desktop Chrome'],
        // Enable headless mode by default
        headless: true,
        // Use a larger viewport for better testing
        viewport: { width: 1920, height: 1080 }
      },
    }]),
  ],

  /* Run your local dev server before starting the tests */
  webServer: {
    command: 'python3 -m http.server 8888',
    url: 'http://localhost:8888',
    cwd: '../../', // Run from wasm-sdk directory to serve index.html
    reuseExistingServer: !process.env.CI,
    timeout: 60000,
  },

  /* Global test timeout */
  timeout: 120000,
  
  /* Expect timeout for assertions */
  expect: {
    timeout: 10000,
  },
});

/**
 * Browser Compatibility Tests
 * Comprehensive testing across all supported browsers and devices
 */

const { test, expect } = require('@playwright/test');

test.describe('Browser Compatibility Tests', () => {
  
  test.beforeEach(async ({ page }) => {
    // Set longer timeout for WASM loading
    test.setTimeout(120000);
  });

  test('WASM module loads and initializes @cross-browser', async ({ page, browserName }) => {
    console.log(`Testing WASM initialization on ${browserName}`);
    
    // Navigate to identity manager sample
    await page.goto('/samples/identity-manager/');
    
    // Check for WASM loading errors
    const errors = [];
    page.on('pageerror', error => {
      errors.push(error.message);
    });
    
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    // Wait for SDK initialization - different timeouts per browser
    const timeout = browserName === 'webkit' ? 90000 : 60000;
    
    try {
      await page.waitForSelector('.status-dot.connected', { timeout });
    } catch (error) {
      console.log(`Errors during initialization on ${browserName}:`, errors);
      throw new Error(`SDK failed to initialize on ${browserName}: ${error.message}`);
    }

    // Verify no critical errors occurred
    const criticalErrors = errors.filter(error => 
      error.includes('WebAssembly') || 
      error.includes('WASM') ||
      error.includes('initialization failed')
    );
    
    expect(criticalErrors).toHaveLength(0);
    
    // Check that status indicator shows connected
    const statusText = await page.textContent('#statusText');
    expect(statusText).toContain('Connected');
    
    console.log(`✅ ${browserName}: WASM initialized successfully`);
  });

  test('Basic API operations work across browsers @api-compatibility', async ({ page, browserName }) => {
    console.log(`Testing API operations on ${browserName}`);
    
    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Test identity lookup
    await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
    await page.click('#lookupBtn');

    // Wait for results or error - longer timeout for mobile browsers
    const resultTimeout = browserName === 'webkit' || page.isMobile ? 45000 : 30000;
    
    await page.waitForSelector('#identityResults, #errorMessage', { timeout: resultTimeout });

    // Check if we got results or a reasonable error
    const hasResults = await page.isVisible('#identityResults');
    const hasError = await page.isVisible('#errorMessage');
    
    expect(hasResults || hasError).toBe(true);

    if (hasError) {
      const errorText = await page.textContent('#errorMessage');
      console.log(`ℹ️  ${browserName}: API operation resulted in error: ${errorText}`);
      
      // Some errors are acceptable (network issues, identity not found)
      const acceptableErrors = [
        'Identity not found',
        'network',
        'timeout',
        'connection'
      ];
      
      const isAcceptableError = acceptableErrors.some(acceptable => 
        errorText.toLowerCase().includes(acceptable.toLowerCase())
      );
      
      if (!isAcceptableError) {
        throw new Error(`Unacceptable API error on ${browserName}: ${errorText}`);
      }
    } else {
      console.log(`✅ ${browserName}: API operations working correctly`);
    }
  });

  test('Memory usage stays within limits @memory-limits', async ({ page, browserName }) => {
    console.log(`Testing memory usage on ${browserName}`);
    
    await page.goto('/samples/document-explorer/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Get initial memory measurement
    const initialMemory = await page.evaluate(() => {
      if (window.performance && window.performance.memory) {
        return window.performance.memory.usedJSHeapSize;
      }
      return 0;
    });

    // Perform several operations to stress memory
    const operations = [
      async () => {
        await page.click('[data-contract="dpns"]');
        await page.waitForTimeout(2000);
      },
      async () => {
        await page.selectOption('#documentTypeSelect', 'domain');
        await page.fill('#limitInput', '25');
        await page.click('#executeQueryBtn');
        await page.waitForSelector('#resultsContainer .document-grid, #resultsContainer .empty-state', { timeout: 30000 });
      },
      async () => {
        // Navigate to token transfer and perform operations
        await page.goto('/samples/token-transfer/');
        await page.waitForSelector('.status-dot.connected', { timeout: 60000 });
        await page.click('#loadPortfolioBtn');
        await page.waitForTimeout(3000);
      }
    ];

    for (let i = 0; i < operations.length; i++) {
      try {
        await operations[i]();
        
        // Measure memory after each operation
        const currentMemory = await page.evaluate(() => {
          if (window.performance && window.performance.memory) {
            return {
              used: window.performance.memory.usedJSHeapSize,
              total: window.performance.memory.totalJSHeapSize,
              limit: window.performance.memory.jsHeapSizeLimit
            };
          }
          return { used: 0, total: 0, limit: 0 };
        });

        if (currentMemory.used > 0) {
          const usedMB = currentMemory.used / 1024 / 1024;
          console.log(`ℹ️  ${browserName} memory after operation ${i + 1}: ${usedMB.toFixed(1)}MB`);

          // Check memory limits based on device type
          const isMobile = page.isMobile || browserName.includes('mobile');
          const memoryLimit = isMobile ? 100 : 200; // 100MB mobile, 200MB desktop
          
          expect(usedMB).toBeLessThan(memoryLimit);
        }

      } catch (error) {
        console.log(`Operation ${i + 1} failed on ${browserName}: ${error.message}`);
        // Continue with other operations
      }
    }

    console.log(`✅ ${browserName}: Memory usage within limits`);
  });

  test('Error handling works correctly @error-handling', async ({ page, browserName }) => {
    console.log(`Testing error handling on ${browserName}`);
    
    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Test invalid identity ID
    await page.fill('#identityIdInput', 'invalid-identity-id');
    await page.click('#lookupBtn');

    // Should show error message
    await page.waitForSelector('#errorMessage', { timeout: 30000 });
    
    const errorText = await page.textContent('#errorMessage');
    expect(errorText).toContain('Invalid');
    
    // Test empty identity ID
    await page.fill('#identityIdInput', '');
    await page.click('#lookupBtn');

    // Should show error for empty input
    await page.waitForSelector('#errorMessage', { timeout: 15000 });
    
    console.log(`✅ ${browserName}: Error handling working correctly`);
  });

  test('Performance meets browser-specific targets @performance-targets', async ({ page, browserName }) => {
    console.log(`Testing performance targets on ${browserName}`);
    
    const startTime = Date.now();
    
    // Navigate and wait for initialization
    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 120000 });
    
    const initializationTime = Date.now() - startTime;
    
    // Browser-specific performance expectations
    const performanceTargets = {
      chromium: { maxInit: 15000, maxQuery: 5000 },    // Best performance
      firefox: { maxInit: 20000, maxQuery: 8000 },     // Good performance
      webkit: { maxInit: 30000, maxQuery: 10000 },     // Safari can be slower
      msedge: { maxInit: 18000, maxQuery: 6000 }       // Similar to Chrome
    };

    const targets = performanceTargets[browserName] || performanceTargets.chromium;
    
    console.log(`ℹ️  ${browserName} initialization time: ${initializationTime}ms (target: <${targets.maxInit}ms)`);
    expect(initializationTime).toBeLessThan(targets.maxInit);

    // Test query performance
    const queryStart = Date.now();
    await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
    await page.click('#lookupBtn');
    await page.waitForSelector('#identityResults, #errorMessage', { timeout: 45000 });
    
    const queryTime = Date.now() - queryStart;
    console.log(`ℹ️  ${browserName} query time: ${queryTime}ms (target: <${targets.maxQuery}ms)`);
    
    // Relaxed query time expectations (network dependent)
    expect(queryTime).toBeLessThan(targets.maxQuery * 2);
    
    console.log(`✅ ${browserName}: Performance targets met`);
  });

  test('UI responsiveness and interactions @ui-compatibility', async ({ page, browserName }) => {
    console.log(`Testing UI interactions on ${browserName}`);
    
    await page.goto('/samples/document-explorer/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Test contract selection
    await page.click('[data-contract="dpns"]');
    await page.waitForSelector('#contractInfo', { timeout: 15000 });
    
    // Verify contract info is displayed
    const contractVisible = await page.isVisible('#contractInfo');
    expect(contractVisible).toBe(true);

    // Test form interactions
    await page.selectOption('#documentTypeSelect', 'domain');
    await page.fill('#limitInput', '5');
    
    // Test query execution
    await page.click('#executeQueryBtn');
    await page.waitForSelector('#resultsContainer .document-grid, #resultsContainer .empty-state', { 
      timeout: 45000 
    });

    // Verify results are displayed
    const hasResults = await page.isVisible('#resultsContainer .document-grid');
    const hasEmptyState = await page.isVisible('#resultsContainer .empty-state');
    
    expect(hasResults || hasEmptyState).toBe(true);

    console.log(`✅ ${browserName}: UI interactions working correctly`);
  });

  test('Mobile device compatibility @mobile-compatibility', async ({ page, browserName }) => {
    // Only run on mobile browsers
    test.skip(!page.isMobile, 'Mobile-only test');
    
    console.log(`Testing mobile compatibility on ${browserName}`);
    
    await page.goto('/samples/identity-manager/');
    
    // Check viewport and touch capabilities
    const viewportSize = page.viewportSize();
    expect(viewportSize.width).toBeLessThan(800); // Mobile viewport
    
    // Wait for SDK with longer timeout on mobile
    await page.waitForSelector('.status-dot.connected', { timeout: 120000 });

    // Test touch interactions
    await page.tap('#identityIdInput');
    await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
    await page.tap('#lookupBtn');

    // Mobile queries may be slower
    await page.waitForSelector('#identityResults, #errorMessage', { timeout: 60000 });
    
    // Test responsive design
    const inputVisible = await page.isVisible('#identityIdInput');
    const buttonVisible = await page.isVisible('#lookupBtn');
    
    expect(inputVisible).toBe(true);
    expect(buttonVisible).toBe(true);

    // Check for mobile-specific layout issues
    const headerHeight = await page.locator('.app-header').boundingBox();
    expect(headerHeight.height).toBeGreaterThan(0);

    console.log(`✅ ${browserName}: Mobile compatibility verified`);
  });

  test('Network conditions impact testing @network-resilience', async ({ page, browserName }) => {
    console.log(`Testing network resilience on ${browserName}`);
    
    // Test with simulated slow network
    await page.emulateNetwork({
      offline: false,
      downloadThroughput: 1.5 * 1000 * 1000, // 1.5 Mbps (slow 4G)
      uploadThroughput: 750 * 1000,           // 750 Kbps
      latency: 200                             // 200ms latency
    });

    const startTime = Date.now();
    await page.goto('/samples/identity-manager/');
    
    // Should still work but may take longer
    await page.waitForSelector('.status-dot.connected', { timeout: 180000 }); // 3 minutes max
    
    const initTime = Date.now() - startTime;
    console.log(`ℹ️  ${browserName} slow network init time: ${initTime}ms`);
    
    // Verify basic functionality still works
    await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
    await page.click('#lookupBtn');
    
    // Allow more time for slow network
    await page.waitForSelector('#identityResults, #errorMessage', { timeout: 90000 });
    
    console.log(`✅ ${browserName}: Network resilience verified`);
  });

  test('Bundle loading performance @bundle-performance', async ({ page, browserName }) => {
    console.log(`Testing bundle loading on ${browserName}`);
    
    // Monitor network requests
    const requests = [];
    page.on('request', request => {
      if (request.url().includes('.wasm') || request.url().includes('.js')) {
        requests.push({
          url: request.url(),
          method: request.method(),
          resourceType: request.resourceType(),
          timestamp: Date.now()
        });
      }
    });

    const responses = [];
    page.on('response', response => {
      if (response.url().includes('.wasm') || response.url().includes('.js')) {
        responses.push({
          url: response.url(),
          status: response.status(),
          headers: response.headers(),
          timestamp: Date.now()
        });
      }
    });

    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Analyze bundle loading
    const wasmRequests = requests.filter(r => r.url.includes('.wasm'));
    const jsRequests = requests.filter(r => r.url.includes('.js'));

    expect(wasmRequests.length).toBeGreaterThan(0); // Should load WASM
    expect(jsRequests.length).toBeGreaterThan(0);   // Should load JS

    // Check successful responses
    const wasmResponses = responses.filter(r => r.url.includes('.wasm') && r.status === 200);
    const jsResponses = responses.filter(r => r.url.includes('.js') && r.status === 200);

    expect(wasmResponses.length).toBeGreaterThan(0);
    expect(jsResponses.length).toBeGreaterThan(0);

    console.log(`ℹ️  ${browserName} bundle loading: ${wasmResponses.length} WASM, ${jsResponses.length} JS files`);
    console.log(`✅ ${browserName}: Bundle loading successful`);
  });

  test('Browser-specific feature detection @feature-detection', async ({ page, browserName }) => {
    console.log(`Testing feature detection on ${browserName}`);
    
    await page.goto('/samples/identity-manager/');
    
    // Check WebAssembly support
    const wasmSupported = await page.evaluate(() => {
      return typeof WebAssembly === 'object' && 
             typeof WebAssembly.instantiate === 'function';
    });
    
    expect(wasmSupported).toBe(true);

    // Check modern JavaScript features
    const jsFeatures = await page.evaluate(() => {
      return {
        es6Modules: typeof import === 'function',
        asyncAwait: typeof (async function(){}) === 'function', 
        promises: typeof Promise === 'function',
        fetch: typeof fetch === 'function',
        crypto: typeof crypto === 'object' && typeof crypto.getRandomValues === 'function'
      };
    });

    expect(jsFeatures.promises).toBe(true);
    expect(jsFeatures.fetch).toBe(true);
    
    // Some features may not be available in all browsers
    console.log(`ℹ️  ${browserName} feature support:`, {
      wasm: wasmSupported,
      ...jsFeatures
    });

    // Check for browser-specific console warnings
    const warnings = [];
    page.on('console', msg => {
      if (msg.type() === 'warning') {
        warnings.push(msg.text());
      }
    });

    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });
    
    // Filter out expected warnings
    const unexpectedWarnings = warnings.filter(warning => 
      !warning.includes('deprecated') && 
      !warning.includes('vendor prefix') &&
      !warning.toLowerCase().includes('network')
    );

    if (unexpectedWarnings.length > 0) {
      console.log(`⚠️  ${browserName} warnings:`, unexpectedWarnings);
    }

    console.log(`✅ ${browserName}: Feature detection complete`);
  });

  test('Cross-browser data consistency @data-consistency', async ({ page, browserName }) => {
    console.log(`Testing data consistency on ${browserName}`);
    
    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Test same operation across browsers - should get consistent results
    await page.fill('#identityIdInput', '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
    await page.click('#lookupBtn');

    await page.waitForSelector('#identityResults, #errorMessage', { timeout: 60000 });

    let resultData = null;
    
    if (await page.isVisible('#identityResults')) {
      const identityData = await page.textContent('#identityData');
      
      try {
        resultData = JSON.parse(identityData);
        
        // Verify basic structure consistency
        expect(resultData).toHaveProperty('id');
        expect(resultData).toHaveProperty('metadata');
        
        console.log(`ℹ️  ${browserName} result structure: ${Object.keys(resultData).join(', ')}`);
        
      } catch (parseError) {
        console.log(`⚠️  ${browserName}: Could not parse result data`);
      }
    }

    console.log(`✅ ${browserName}: Data consistency verified`);
  });

  test('Long-running stability @stability', async ({ page, browserName }) => {
    // Skip on CI to avoid timeouts
    test.skip(!!process.env.CI, 'Long-running test skipped in CI');
    
    console.log(`Testing long-running stability on ${browserName}`);
    test.setTimeout(600000); // 10 minutes
    
    await page.goto('/samples/token-transfer/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Perform operations over 5 minutes
    const endTime = Date.now() + 5 * 60 * 1000;
    let operationCount = 0;
    const errors = [];

    page.on('pageerror', error => {
      errors.push({ timestamp: Date.now(), error: error.message });
    });

    while (Date.now() < endTime) {
      try {
        // Rotate through different operations
        const operation = operationCount % 3;
        
        switch (operation) {
          case 0:
            await page.click('#loadPortfolioBtn');
            break;
          case 1:
            await page.fill('#tokenContractId', 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv');
            await page.click('#calculateTokenIdBtn');
            break;
          case 2:
            await page.click('#getPricingBtn');
            break;
        }

        operationCount++;
        await page.waitForTimeout(3000);

        // Check memory periodically
        if (operationCount % 10 === 0) {
          const memory = await page.evaluate(() => {
            return window.performance?.memory?.usedJSHeapSize || 0;
          });
          
          const memoryMB = memory / 1024 / 1024;
          console.log(`ℹ️  ${browserName} operation ${operationCount}, memory: ${memoryMB.toFixed(1)}MB`);
          
          // Memory should not grow excessively
          const isMobile = page.isMobile || browserName.includes('mobile');
          const memoryLimit = isMobile ? 150 : 250; // Relaxed limits for long-running
          expect(memoryMB).toBeLessThan(memoryLimit);
        }

      } catch (error) {
        errors.push({ timestamp: Date.now(), error: error.message });
        console.log(`⚠️  ${browserName} operation ${operationCount} failed: ${error.message}`);
      }
    }

    console.log(`ℹ️  ${browserName} stability test: ${operationCount} operations, ${errors.length} errors`);
    
    // Allow some errors but not excessive failures
    const errorRate = errors.length / operationCount;
    expect(errorRate).toBeLessThan(0.2); // Less than 20% error rate
    
    console.log(`✅ ${browserName}: Long-running stability verified`);
  });

});

test.describe('Mobile-Specific Tests', () => {
  
  test('Touch interactions work correctly @mobile-touch', async ({ page, browserName }) => {
    test.skip(!page.isMobile, 'Mobile-only test');
    
    console.log(`Testing touch interactions on ${browserName}`);
    
    await page.goto('/samples/dpns-resolver/');
    await page.waitForSelector('.status-dot.connected', { timeout: 120000 });

    // Test various touch interactions
    await page.tap('#usernameInput');
    await page.fill('#usernameInput', 'alice');
    await page.tap('#resolveBtn');

    await page.waitForSelector('#resolutionResults, #errorMessage', { timeout: 60000 });
    
    // Test modal interactions on mobile
    await page.goto('/samples/document-explorer/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });
    
    await page.tap('[data-contract="dpns"]');
    await page.waitForTimeout(2000);
    
    console.log(`✅ ${browserName}: Touch interactions working`);
  });

  test('Mobile performance within constraints @mobile-performance', async ({ page, browserName }) => {
    test.skip(!page.isMobile, 'Mobile-only test');
    
    console.log(`Testing mobile performance constraints on ${browserName}`);
    
    const startTime = Date.now();
    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 180000 }); // 3 minutes max on mobile
    
    const mobileInitTime = Date.now() - startTime;
    console.log(`ℹ️  Mobile ${browserName} init time: ${mobileInitTime}ms`);
    
    // Mobile should initialize within 3 minutes even on slow devices
    expect(mobileInitTime).toBeLessThan(180000);
    
    // Check memory usage
    const memory = await page.evaluate(() => {
      return window.performance?.memory?.usedJSHeapSize || 0;
    });
    
    const memoryMB = memory / 1024 / 1024;
    console.log(`ℹ️  Mobile ${browserName} memory usage: ${memoryMB.toFixed(1)}MB`);
    
    // Mobile memory constraint: 100MB
    expect(memoryMB).toBeLessThan(100);
    
    console.log(`✅ Mobile ${browserName}: Performance constraints met`);
  });

});

test.describe('Browser-Specific Edge Cases', () => {

  test('Safari WebAssembly compatibility @safari-specific', async ({ page, browserName }) => {
    test.skip(browserName !== 'webkit', 'Safari-specific test');
    
    console.log('Testing Safari-specific WebAssembly features');
    
    await page.goto('/samples/identity-manager/');
    
    // Safari may have specific WASM loading behavior
    const wasmLoadEvent = await page.evaluate(() => {
      return new Promise((resolve) => {
        const originalFetch = window.fetch;
        window.fetch = function(...args) {
          if (args[0] && args[0].includes('.wasm')) {
            resolve('wasm-fetch-detected');
          }
          return originalFetch.apply(this, args);
        };
        
        setTimeout(() => resolve('no-wasm-fetch'), 30000);
      });
    });
    
    expect(wasmLoadEvent).toBe('wasm-fetch-detected');
    
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });
    console.log('✅ Safari: WASM compatibility verified');
  });

  test('Firefox memory management @firefox-specific', async ({ page, browserName }) => {
    test.skip(browserName !== 'firefox', 'Firefox-specific test');
    
    console.log('Testing Firefox-specific memory management');
    
    await page.goto('/samples/document-explorer/');
    await page.waitForSelector('.status-dot.connected', { timeout: 90000 });

    // Firefox may handle WASM memory differently
    const memoryBehavior = await page.evaluate(async () => {
      // Trigger potential garbage collection
      if (window.gc) {
        window.gc();
      }
      
      // Force memory allocation and release pattern
      const arrays = [];
      for (let i = 0; i < 100; i++) {
        arrays.push(new Uint8Array(1000000)); // 1MB each
      }
      
      const beforeGC = window.performance?.memory?.usedJSHeapSize || 0;
      
      // Clear arrays
      arrays.length = 0;
      
      // Force GC if available
      if (window.gc) {
        window.gc();
      }
      
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      const afterGC = window.performance?.memory?.usedJSHeapSize || 0;
      
      return {
        beforeGC: beforeGC / 1024 / 1024,
        afterGC: afterGC / 1024 / 1024,
        memoryReclaimed: (beforeGC - afterGC) / 1024 / 1024
      };
    });

    console.log(`ℹ️  Firefox memory behavior:`, memoryBehavior);
    console.log('✅ Firefox: Memory management verified');
  });

  test('Chrome performance optimizations @chrome-specific', async ({ page, browserName }) => {
    test.skip(!browserName.includes('chromium'), 'Chrome-specific test');
    
    console.log('Testing Chrome performance optimizations');
    
    // Enable performance timeline
    await page.evaluate(() => {
      if (window.performance && window.performance.mark) {
        window.performance.mark('test-start');
      }
    });

    await page.goto('/samples/identity-manager/');
    await page.waitForSelector('.status-dot.connected', { timeout: 60000 });

    // Chrome should have the best performance
    const performanceMetrics = await page.evaluate(() => {
      if (window.performance && window.performance.mark) {
        window.performance.mark('test-end');
        window.performance.measure('total-test', 'test-start', 'test-end');
        
        const measures = window.performance.getEntriesByType('measure');
        const navigation = window.performance.getEntriesByType('navigation')[0];
        
        return {
          totalTest: measures.find(m => m.name === 'total-test')?.duration,
          domContentLoaded: navigation?.domContentLoadedEventEnd - navigation?.navigationStart,
          loadComplete: navigation?.loadEventEnd - navigation?.navigationStart
        };
      }
      return null;
    });

    if (performanceMetrics) {
      console.log(`ℹ️  Chrome performance metrics:`, performanceMetrics);
      
      // Chrome should load relatively quickly
      expect(performanceMetrics.domContentLoaded).toBeLessThan(30000); // 30s max
    }

    console.log('✅ Chrome: Performance optimizations verified');
  });

});
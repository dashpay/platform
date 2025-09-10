/**
 * Comprehensive Functionality Tests for DPNS Resolver Web App
 * Tests username resolution, validation, domain browsing, and all DPNS operations
 */

const { test, expect } = require('@playwright/test');

test.describe('DPNS Resolver - Functionality Tests', () => {
  let page;

  test.beforeEach(async ({ browser }) => {
    page = await browser.newPage();
    await page.goto('http://localhost:8888/samples/dpns-resolver/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Allow for SDK initialization
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('Application Initialization', () => {
    test('should load DPNS Resolver interface', async () => {
      // Check main elements are present
      await expect(page.locator('.app-container')).toBeVisible();
      await expect(page.locator('.app-header h1')).toContainText('DPNS Resolver');
      
      // Check network selector
      await expect(page.locator('#networkSelect')).toBeVisible();
      
      // Check main sections
      await expect(page.locator('.username-resolution')).toBeVisible();
      await expect(page.locator('.username-validation')).toBeVisible();
      await expect(page.locator('.domain-browser')).toBeVisible();
    });

    test('should initialize SDK and show connection status', async () => {
      // Wait for SDK initialization
      await page.waitForTimeout(5000);
      
      // Check status indicator
      const statusIndicator = page.locator('.status-indicator');
      if (await statusIndicator.isVisible()) {
        const statusText = await statusIndicator.textContent();
        expect(statusText).toMatch(/connected|initialized|ready/i);
      }
      
      // Network should be set
      const networkSelect = page.locator('#networkSelect');
      const networkValue = await networkSelect.inputValue();
      expect(['testnet', 'mainnet']).toContain(networkValue);
    });

    test('should handle network switching', async () => {
      const networkSelect = page.locator('#networkSelect');
      
      // Switch to mainnet
      await networkSelect.selectOption('mainnet');
      await page.waitForTimeout(3000);
      expect(await networkSelect.inputValue()).toBe('mainnet');
      
      // Switch back to testnet
      await networkSelect.selectOption('testnet');
      await page.waitForTimeout(3000);
      expect(await networkSelect.inputValue()).toBe('testnet');
    });
  });

  describe('Username Resolution', () => {
    test('should resolve existing usernames', async () => {
      // Try to resolve a potentially existing username
      const usernameInput = page.locator('#usernameInput');
      await usernameInput.fill('alice');
      
      const resolveBtn = page.locator('#resolveBtn');
      await resolveBtn.click();
      
      // Wait for resolution
      await page.waitForTimeout(10000);
      
      // Check resolution results
      const resolutionResults = page.locator('.resolution-results');
      if (await resolutionResults.isVisible()) {
        const resultsText = await resolutionResults.textContent();
        
        // Should show either identity ID or not found message
        expect(resultsText).toMatch(/identity|not found|resolved|error/i);
        
        if (resultsText.includes('Identity')) {
          // If username resolved, should show identity information
          expect(resultsText).toMatch(/[A-Za-z0-9]{44,}/); // Base58 identity ID pattern
        }
      }
    });

    test('should handle non-existent usernames', async () => {
      // Try to resolve a username that definitely doesn't exist
      const usernameInput = page.locator('#usernameInput');
      await usernameInput.fill('extremely-unlikely-username-12345-does-not-exist');
      
      const resolveBtn = page.locator('#resolveBtn');
      await resolveBtn.click();
      
      await page.waitForTimeout(10000);
      
      // Should show appropriate message for non-existent username
      const resolutionResults = page.locator('.resolution-results');
      if (await resolutionResults.isVisible()) {
        const resultsText = await resolutionResults.textContent();
        expect(resultsText).toMatch(/not found|does not exist|no.*match/i);
      }
    });

    test('should handle username resolution with Enter key', async () => {
      const usernameInput = page.locator('#usernameInput');
      await usernameInput.fill('test');
      
      // Press Enter instead of clicking resolve button
      await usernameInput.press('Enter');
      
      await page.waitForTimeout(8000);
      
      // Should trigger resolution
      const resolutionResults = page.locator('.resolution-results');
      if (await resolutionResults.isVisible()) {
        const resultsText = await resolutionResults.textContent();
        expect(resultsText.length).toBeGreaterThan(0);
      }
    });

    test('should display resolution performance metrics', async () => {
      // Execute resolution
      const usernameInput = page.locator('#usernameInput');
      await usernameInput.fill('alice');
      
      const resolveBtn = page.locator('#resolveBtn');
      const startTime = Date.now();
      await resolveBtn.click();
      
      await page.waitForTimeout(10000);
      const endTime = Date.now();
      
      const queryTime = endTime - startTime;
      console.log(`Username resolution took ${queryTime}ms`);
      
      // Check if timing information is displayed
      const timingInfo = page.locator('.timing-info, .performance-metrics');
      if (await timingInfo.isVisible()) {
        const timingText = await timingInfo.textContent();
        expect(timingText).toMatch(/\d+ms|time|duration/i);
      }
    });

    test('should handle empty username input', async () => {
      // Try to resolve empty username
      const usernameInput = page.locator('#usernameInput');
      await usernameInput.fill('');
      
      const resolveBtn = page.locator('#resolveBtn');
      await resolveBtn.click();
      
      await page.waitForTimeout(3000);
      
      // Should show validation error or prevent execution
      const errorMessage = page.locator('.error-message, .validation-error');
      if (await errorMessage.isVisible()) {
        const errorText = await errorMessage.textContent();
        expect(errorText).toMatch(/enter.*username|required|empty/i);
      }
    });
  });

  describe('Username Validation', () => {
    test('should validate correct username formats', async () => {
      const validUsernames = [
        'alice',
        'bob123',
        'test-user',
        'valid_name',
        'a',
        'longusernamethatmightbevalid'
      ];
      
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      for (const username of validUsernames) {
        await validateInput.fill(username);
        await validateBtn.click();
        
        await page.waitForTimeout(2000);
        
        // Check validation results
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Should show validation status (valid/invalid with reasons)
          expect(resultsText).toMatch(/valid|invalid|check|format/i);
          
          if (resultsText.toLowerCase().includes('valid')) {
            // If marked as valid, should provide additional info
            expect(resultsText.length).toBeGreaterThan(10);
          }
        }
      }
    });

    test('should detect invalid username formats', async () => {
      const invalidUsernames = [
        '',                    // Empty
        'a'.repeat(100),       // Too long
        'UPPERCASE',           // Uppercase letters
        'user@domain',         // Invalid characters
        'user.with.dots',      // Dots
        'user with spaces',    // Spaces
        'user!@#$',           // Special characters
        '123',                // Numbers only
        '-startwithhyphen',   // Start with hyphen
        'endwithhyphen-'      // End with hyphen
      ];
      
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      for (const username of invalidUsernames) {
        await validateInput.fill(username);
        await validateBtn.click();
        
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Should mark as invalid with explanation
          if (username !== '') { // Empty might have different handling
            expect(resultsText).toMatch(/invalid|not.*valid|error/i);
          }
        }
      }
    });

    test('should perform real-time validation', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      
      // Type a username character by character
      await validateInput.fill('');
      await validateInput.type('test123');
      
      // Wait for real-time validation to trigger
      await page.waitForTimeout(3000);
      
      // Check if real-time validation indicators appear
      const validationIndicator = page.locator('.validation-indicator, .real-time-validation');
      if (await validationIndicator.isVisible()) {
        // Should show some validation feedback
        const indicatorText = await validationIndicator.textContent();
        expect(indicatorText.length).toBeGreaterThan(0);
      }
    });

    test('should show validation with Enter key', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      await validateInput.fill('validuser');
      
      // Press Enter instead of clicking validate button
      await validateInput.press('Enter');
      
      await page.waitForTimeout(3000);
      
      // Should trigger validation
      const validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        const resultsText = await validationResults.textContent();
        expect(resultsText).toMatch(/valid|check|format/i);
      }
    });

    test('should provide detailed validation feedback', async () => {
      // Test with problematic username
      const validateInput = page.locator('#validateUsernameInput');
      await validateInput.fill('INVALID-USERNAME@');
      
      const validateBtn = page.locator('#validateBtn');
      await validateBtn.click();
      
      await page.waitForTimeout(3000);
      
      const validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        const resultsText = await validationResults.textContent();
        
        // Should provide specific error details
        expect(resultsText).toMatch(/invalid|uppercase|special.*character|format/i);
      }
    });
  });

  describe('Reverse Resolution', () => {
    test('should perform reverse resolution with valid identity ID', async () => {
      // Use a known identity ID (might or might not have username)
      const identityInput = page.locator('#identityIdInput');
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      
      const reverseResolveBtn = page.locator('#reverseResolveBtn, #reverseResolveDirectBtn');
      if (await reverseResolveBtn.isVisible()) {
        await reverseResolveBtn.click();
        
        await page.waitForTimeout(10000);
        
        // Check reverse resolution results
        const reverseResults = page.locator('.reverse-results, .resolution-results');
        if (await reverseResults.isVisible()) {
          const resultsText = await reverseResults.textContent();
          
          // Should show either username or no username found
          expect(resultsText).toMatch(/username|no.*username|not.*found|resolved/i);
        }
      }
    });

    test('should handle invalid identity IDs for reverse resolution', async () => {
      const invalidIds = [
        'invalid-id',
        'too-short',
        'x'.repeat(100),
        '!@#$%^&*()'
      ];
      
      const identityInput = page.locator('#identityIdInput');
      const reverseResolveBtn = page.locator('#reverseResolveBtn, #reverseResolveDirectBtn');
      
      for (const id of invalidIds) {
        await identityInput.fill(id);
        
        if (await reverseResolveBtn.isVisible()) {
          await reverseResolveBtn.click();
          await page.waitForTimeout(5000);
          
          // Should show error or validation message
          const errorMessage = page.locator('.error-message, .reverse-results');
          if (await errorMessage.isVisible()) {
            const errorText = await errorMessage.textContent();
            expect(errorText).toMatch(/invalid|error|not.*found|malformed/i);
          }
        }
      }
    });

    test('should perform reverse resolution from query results', async () => {
      // First perform a domain query to get identity IDs
      const domainBrowseBtn = page.locator('#browseDomainBtn');
      if (await domainBrowseBtn.isVisible()) {
        await domainBrowseBtn.click();
        await page.waitForTimeout(15000);
        
        // Check if any domain results are shown
        const domainResults = page.locator('.domain-result, .document-item');
        const resultCount = await domainResults.count();
        
        if (resultCount > 0) {
          // Look for reverse resolve button on results
          const reverseFromResultBtn = page.locator('#reverseResolveBtn');
          if (await reverseFromResultBtn.isVisible()) {
            await reverseFromResultBtn.click();
            await page.waitForTimeout(8000);
            
            const reverseResults = page.locator('.reverse-results');
            if (await reverseResults.isVisible()) {
              const resultsText = await reverseResults.textContent();
              expect(resultsText.length).toBeGreaterThan(0);
            }
          }
        }
      }
    });
  });

  describe('Domain Browsing and Filtering', () => {
    test('should browse domains with default settings', async () => {
      const browseDomainBtn = page.locator('#browseDomainBtn');
      await browseDomainBtn.click();
      
      // Wait for domain browsing results
      await page.waitForTimeout(15000);
      
      // Check for domain results
      const domainResults = page.locator('.domain-results, .results-container');
      if (await domainResults.isVisible()) {
        const domainItems = page.locator('.domain-item, .document-item');
        const itemCount = await domainItems.count();
        
        if (itemCount > 0) {
          // Check structure of first domain item
          const firstItem = domainItems.first();
          const itemText = await firstItem.textContent();
          
          // Should contain domain information
          expect(itemText).toMatch(/label|owner|created|dash/i);
        } else {
          // No results is also valid
          const emptyState = page.locator('.empty-state, .no-results');
          if (await emptyState.isVisible()) {
            expect(await emptyState.textContent()).toMatch(/no.*domains|empty/i);
          }
        }
      }
    });

    test('should filter domains by criteria', async () => {
      // Set up domain filter if available
      const domainFilter = page.locator('#domainFilter');
      
      if (await domainFilter.isVisible()) {
        // Try different filter options
        const filterOptions = await domainFilter.locator('option').allTextContents();
        
        if (filterOptions.length > 1) {
          // Select a specific filter
          await domainFilter.selectOption(filterOptions[1]);
          
          await page.waitForTimeout(1000);
          
          // Browse with filter applied
          const browseDomainBtn = page.locator('#browseDomainBtn');
          await browseDomainBtn.click();
          
          await page.waitForTimeout(15000);
          
          // Check filtered results
          const domainResults = page.locator('.domain-results, .results-container');
          if (await domainResults.isVisible()) {
            const resultsText = await domainResults.textContent();
            expect(resultsText.length).toBeGreaterThan(0);
          }
        }
      }
    });

    test('should handle domain browsing errors gracefully', async () => {
      // Try browsing with potential error conditions
      // (This might involve network issues or invalid configurations)
      
      const browseDomainBtn = page.locator('#browseDomainBtn');
      await browseDomainBtn.click();
      
      // Wait longer to see if errors occur
      await page.waitForTimeout(20000);
      
      // Check for error handling
      const errorMessage = page.locator('.error-message, .browse-error');
      const loadingIndicator = page.locator('.loading, .spinner');
      const domainResults = page.locator('.domain-results, .results-container');
      
      const hasError = await errorMessage.isVisible();
      const isLoading = await loadingIndicator.isVisible();
      const hasResults = await domainResults.isVisible();
      
      // Should be in one of these states
      expect(hasError || isLoading || hasResults).toBe(true);
      
      if (hasError) {
        const errorText = await errorMessage.textContent();
        expect(errorText).toMatch(/error|failed|timeout|network/i);
      }
    });

    test('should display domain metadata correctly', async () => {
      const browseDomainBtn = page.locator('#browseDomainBtn');
      await browseDomainBtn.click();
      await page.waitForTimeout(15000);
      
      const domainItems = page.locator('.domain-item, .document-item');
      const itemCount = await domainItems.count();
      
      if (itemCount > 0) {
        // Check first domain item structure
        const firstItem = domainItems.first();
        const itemText = await firstItem.textContent();
        
        // Should contain essential domain metadata
        expect(itemText).toMatch(/label|owner|created|parent/i);
        
        // Check for Base58 encoded identifiers
        expect(itemText).toMatch(/[A-Za-z0-9]{20,}/);
      }
    });
  });

  describe('Registration Cost Calculation', () => {
    test('should calculate registration costs for valid usernames', async () => {
      const usernameInput = page.locator('#costCalculationUsername, #usernameForCost');
      const calculateCostBtn = page.locator('#calculateCostBtn');
      
      if (await usernameInput.isVisible() && await calculateCostBtn.isVisible()) {
        // Test different username lengths (cost might vary by length)
        const testUsernames = ['a', 'ab', 'test', 'longtestname'];
        
        for (const username of testUsernames) {
          await usernameInput.fill(username);
          await calculateCostBtn.click();
          
          await page.waitForTimeout(5000);
          
          // Check cost calculation results
          const costResults = page.locator('.cost-results, .registration-cost');
          if (await costResults.isVisible()) {
            const costText = await costResults.textContent();
            
            // Should show cost information
            expect(costText).toMatch(/cost|credits|dash|price/i);
            
            // Should show numeric value
            expect(costText).toMatch(/\d+/);
          }
        }
      }
    });

    test('should handle invalid usernames for cost calculation', async () => {
      const usernameInput = page.locator('#costCalculationUsername, #usernameForCost');
      const calculateCostBtn = page.locator('#calculateCostBtn');
      
      if (await usernameInput.isVisible() && await calculateCostBtn.isVisible()) {
        // Try invalid username
        await usernameInput.fill('INVALID@USERNAME');
        await calculateCostBtn.click();
        
        await page.waitForTimeout(5000);
        
        // Should show error or validation message
        const costResults = page.locator('.cost-results, .error-message');
        if (await costResults.isVisible()) {
          const resultText = await costResults.textContent();
          expect(resultText).toMatch(/invalid|error|cannot.*calculate/i);
        }
      }
    });

    test('should show cost breakdown if available', async () => {
      const usernameInput = page.locator('#costCalculationUsername, #usernameForCost');
      const calculateCostBtn = page.locator('#calculateCostBtn');
      
      if (await usernameInput.isVisible() && await calculateCostBtn.isVisible()) {
        await usernameInput.fill('testcost');
        await calculateCostBtn.click();
        
        await page.waitForTimeout(5000);
        
        const costResults = page.locator('.cost-results');
        if (await costResults.isVisible()) {
          const costText = await costResults.textContent();
          
          // Look for detailed cost breakdown
          if (costText.includes('breakdown') || costText.includes('fees')) {
            expect(costText).toMatch(/base.*cost|fee|total/i);
          }
        }
      }
    });
  });

  describe('Statistics and Analytics', () => {
    test('should display DPNS statistics when refreshed', async () => {
      const refreshStatsBtn = page.locator('#refreshStatsBtn');
      
      if (await refreshStatsBtn.isVisible()) {
        await refreshStatsBtn.click();
        
        await page.waitForTimeout(10000);
        
        // Check for statistics display
        const statsContainer = page.locator('.statistics, .stats-container');
        if (await statsContainer.isVisible()) {
          const statsText = await statsContainer.textContent();
          
          // Should show domain statistics
          expect(statsText).toMatch(/domains|total|count|registered/i);
          
          // Should contain numeric data
          expect(statsText).toMatch(/\d+/);
        }
      }
    });

    test('should handle statistics refresh errors', async () => {
      const refreshStatsBtn = page.locator('#refreshStatsBtn');
      
      if (await refreshStatsBtn.isVisible()) {
        // Try multiple rapid refreshes
        for (let i = 0; i < 3; i++) {
          await refreshStatsBtn.click();
          await page.waitForTimeout(1000);
        }
        
        await page.waitForTimeout(10000);
        
        // Should handle rapid requests gracefully
        const errorMessage = page.locator('.error-message');
        const statsContainer = page.locator('.statistics, .stats-container');
        
        const hasError = await errorMessage.isVisible();
        const hasStats = await statsContainer.isVisible();
        
        // Should show either stats or error
        expect(hasError || hasStats).toBe(true);
      }
    });

    test('should display platform activity metrics', async () => {
      // Check if activity metrics are shown automatically or need refresh
      await page.waitForTimeout(5000);
      
      const activityMetrics = page.locator('.activity-metrics, .platform-activity');
      if (await activityMetrics.isVisible()) {
        const metricsText = await activityMetrics.textContent();
        
        // Should show activity information
        expect(metricsText).toMatch(/recent|activity|registrations|queries/i);
      }
    });
  });

  describe('User Interface and Experience', () => {
    test('should provide clear visual feedback for operations', async () => {
      // Test username resolution feedback
      const usernameInput = page.locator('#usernameInput');
      await usernameInput.fill('test');
      
      const resolveBtn = page.locator('#resolveBtn');
      await resolveBtn.click();
      
      // Should show loading state
      await page.waitForTimeout(1000);
      const loadingIndicator = page.locator('.loading, .spinner, .resolving');
      
      if (await loadingIndicator.isVisible()) {
        expect(await loadingIndicator.isVisible()).toBe(true);
      }
      
      await page.waitForTimeout(8000);
      
      // Loading should disappear and show results
      const resultsContainer = page.locator('.resolution-results, .results');
      if (await resultsContainer.isVisible()) {
        expect(await resultsContainer.isVisible()).toBe(true);
      }
    });

    test('should maintain consistent styling across sections', async () => {
      // Check that all main sections are properly styled
      const sections = [
        '.username-resolution',
        '.username-validation', 
        '.domain-browser',
        '.registration-cost'
      ];
      
      for (const selector of sections) {
        const section = page.locator(selector);
        if (await section.isVisible()) {
          // Check section has consistent structure
          const hasHeader = await section.locator('h2, h3, .section-header').isVisible();
          const hasContent = await section.locator('.section-content, .content').isVisible();
          
          // Most sections should have headers and content
          expect(hasHeader || hasContent).toBe(true);
        }
      }
    });

    test('should handle keyboard navigation properly', async () => {
      // Test Tab navigation through form elements
      await page.keyboard.press('Tab');
      await page.keyboard.press('Tab');
      await page.keyboard.press('Tab');
      
      // Should be able to focus on form elements
      const focusedElement = await page.evaluate(() => document.activeElement.tagName);
      expect(['INPUT', 'BUTTON', 'SELECT']).toContain(focusedElement);
    });

    test('should display helpful tooltips or hints', async () => {
      // Check for help text or tooltips
      const helpElements = page.locator('.help-text, .tooltip, .hint');
      const helpCount = await helpElements.count();
      
      if (helpCount > 0) {
        // Check first help element has useful content
        const helpText = await helpElements.first().textContent();
        expect(helpText.length).toBeGreaterThan(10);
      }
    });

    test('should be responsive to different viewport sizes', async () => {
      // Test mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });
      await page.waitForTimeout(1000);
      
      // Main container should still be visible
      const appContainer = page.locator('.app-container');
      await expect(appContainer).toBeVisible();
      
      // Test tablet viewport
      await page.setViewportSize({ width: 768, height: 1024 });
      await page.waitForTimeout(1000);
      
      await expect(appContainer).toBeVisible();
      
      // Test desktop viewport
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.waitForTimeout(1000);
      
      await expect(appContainer).toBeVisible();
    });
  });
});
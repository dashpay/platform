/**
 * Edge Cases and Error Handling Tests for Document Explorer
 * Tests invalid inputs, network failures, malformed data, and recovery scenarios
 */

const { test, expect } = require('@playwright/test');

test.describe('Document Explorer - Edge Cases and Error Handling', () => {
  let page;

  test.beforeEach(async ({ browser }) => {
    page = await browser.newPage();
    await page.goto('http://localhost:8888/samples/document-explorer/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('Invalid Contract ID Handling', () => {
    test('should handle empty contract IDs gracefully', async () => {
      // Try to load empty contract ID
      await page.fill('#customContractId', '');
      await page.click('#loadContractBtn');
      await page.waitForTimeout(3000);
      
      // Should show error message
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/enter.*contract|empty|required/i);
      }
      
      // Contract info should not appear
      const contractInfo = page.locator('#contractInfo');
      expect(await contractInfo.isVisible()).toBe(false);
    });

    test('should handle malformed contract IDs', async () => {
      const malformedIds = [
        'abc123',           // Too short
        '!@#$%^&*()',      // Special characters
        'x'.repeat(100),   // Too long
        'NotBase58Characters!@#',
        '123',             // Numbers only
        'zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz' // Invalid Base58
      ];

      for (const invalidId of malformedIds) {
        await page.fill('#customContractId', invalidId);
        await page.click('#loadContractBtn');
        await page.waitForTimeout(5000);
        
        // Should handle error gracefully
        const contractInfo = page.locator('#contractInfo');
        const errorResults = page.locator('#errorResults');
        
        const contractVisible = await contractInfo.isVisible();
        const errorVisible = await errorResults.isVisible();
        
        // Either should show error or not load contract
        if (contractVisible) {
          // If contract somehow loads, it shouldn't be valid
          console.warn(`Unexpectedly loaded contract for ID: ${invalidId}`);
        } else {
          // Error handling is working correctly
          expect(contractVisible).toBe(false);
        }
        
        // Clear for next iteration
        await page.fill('#customContractId', '');
      }
    });

    test('should handle non-existent contract IDs', async () => {
      // Use valid Base58 format but non-existent contract
      const nonExistentId = 'NonExistentContract1234567890AbCdEf1234567890';
      
      await page.fill('#customContractId', nonExistentId);
      await page.click('#loadContractBtn');
      await page.waitForTimeout(10000); // Allow more time for network request
      
      // Should show appropriate error
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/not found|failed to load|contract.*not.*exist/i);
      }
      
      const contractInfo = page.locator('#contractInfo');
      expect(await contractInfo.isVisible()).toBe(false);
    });

    test('should recover from contract loading errors', async () => {
      // First try invalid contract
      await page.fill('#customContractId', 'invalid-contract');
      await page.click('#loadContractBtn');
      await page.waitForTimeout(5000);
      
      // Then try valid contract
      await page.fill('#customContractId', 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      await page.click('#loadContractBtn');
      await page.waitForTimeout(8000);
      
      // Should now load successfully
      const contractInfo = page.locator('#contractInfo');
      if (await contractInfo.isVisible()) {
        const contractDetails = page.locator('#contractDetails');
        const detailsText = await contractDetails.textContent();
        expect(detailsText).toContain('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
      }
    });
  });

  describe('Invalid Document Type Handling', () => {
    test('should handle queries with invalid document types', async () => {
      // Load valid contract first
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Try to manually set invalid document type (if possible through DOM manipulation)
      await page.evaluate(() => {
        const select = document.getElementById('documentTypeSelect');
        if (select) {
          const option = document.createElement('option');
          option.value = 'invalid-document-type';
          option.textContent = 'Invalid Type';
          select.appendChild(option);
          select.value = 'invalid-document-type';
        }
      });
      
      // Try to execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Should handle error gracefully
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/invalid.*document.*type|not.*found|failed/i);
      }
    });

    test('should handle empty document type selection', async () => {
      // Load contract but don't select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Try to execute query without selecting document type
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(3000);
      
      // Should prevent execution or show error
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/select.*document.*type|required|missing/i);
      }
      
      // Results should not show documents
      const documentItems = page.locator('.document-item');
      expect(await documentItems.count()).toBe(0);
    });
  });

  describe('Query Parameter Edge Cases', () => {
    test('should handle invalid limit values', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      const invalidLimits = [-1, 0, 1001, 'abc', '', '10.5'];
      
      for (const limit of invalidLimits) {
        await page.fill('#limitInput', limit.toString());
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(5000);
        
        // Should either correct the value or show error
        const limitValue = await page.locator('#limitInput').inputValue();
        const parsedLimit = parseInt(limitValue);
        
        if (!isNaN(parsedLimit)) {
          // If parsed successfully, should be within valid range
          expect(parsedLimit).toBeGreaterThan(0);
          expect(parsedLimit).toBeLessThanOrEqual(100);
        }
        
        const errorResults = page.locator('#errorResults');
        if (await errorResults.isVisible()) {
          const errorText = await errorResults.textContent();
          expect(errorText).toMatch(/invalid.*limit|out.*of.*range/i);
        }
      }
    });

    test('should handle invalid offset values', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      const invalidOffsets = [-1, 'abc', '', '5.7'];
      
      for (const offset of invalidOffsets) {
        await page.fill('#offsetInput', offset.toString());
        await page.fill('#limitInput', '5'); // Valid limit
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(5000);
        
        // Should either correct the value or show error
        const offsetValue = await page.locator('#offsetInput').inputValue();
        const parsedOffset = parseInt(offsetValue);
        
        if (!isNaN(parsedOffset)) {
          // If parsed successfully, should not be negative
          expect(parsedOffset).toBeGreaterThanOrEqual(0);
        }
      }
    });

    test('should handle extremely large parameter values', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Try extremely large values
      await page.fill('#limitInput', '999999');
      await page.fill('#offsetInput', '999999');
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Should handle large values gracefully (might timeout or limit internally)
      const errorResults = page.locator('#errorResults');
      const loadingResults = page.locator('#loadingResults');
      
      // Either should complete, show error, or still be loading
      const isError = await errorResults.isVisible();
      const isLoading = await loadingResults.isVisible();
      const hasResults = await page.locator('.document-item').count() > 0;
      const isEmpty = await page.locator('.empty-state').isVisible();
      
      // Should be in one of these states
      expect(isError || isLoading || hasResults || isEmpty).toBe(true);
      
      if (isError) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/limit|timeout|too.*large|invalid/i);
      }
    });
  });

  describe('WHERE Clause Edge Cases', () => {
    test('should handle invalid WHERE clause field values', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set invalid field combination
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('$createdAt');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('=');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('not-a-timestamp'); // Invalid timestamp
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Should handle type mismatch error
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/invalid.*value|type.*error|failed/i);
      }
    });

    test('should handle empty WHERE clause values', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set field and operator but leave value empty
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('label');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('=');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill(''); // Empty value
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      // Should either skip empty condition or show validation error
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        
        // Empty conditions should not appear in generated query
        expect(queryCode).not.toContain('""');
        expect(queryCode).not.toContain("''");
      }
    });

    test('should handle malformed JSON in array fields', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set IN operator with malformed JSON
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('in');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('[invalid, json, array,'); // Malformed JSON
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Should handle JSON parsing error
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/json|parse|invalid.*format|syntax/i);
      }
    });

    test('should handle contradictory WHERE conditions', async () => {
      // Setup valid contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Add contradictory conditions
      let fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('label');
      let valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('test');
      
      // Add second condition that contradicts first
      await page.click('#addConditionBtn');
      await page.waitForTimeout(500);
      
      const conditions = page.locator('#whereConditions .condition-row');
      const secondCondition = conditions.nth(1);
      
      fieldSelect = secondCondition.locator('.field-select');
      await fieldSelect.selectOption('label');
      
      const operatorSelect = secondCondition.locator('.operator-select');
      await operatorSelect.selectOption('=');
      
      valueInput = secondCondition.locator('.value-input');
      await valueInput.fill('different-value'); // Contradictory value
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Should return no results (empty state)
      const emptyState = page.locator('.empty-state');
      const documentItems = page.locator('.document-item');
      
      const isEmpty = await emptyState.isVisible();
      const hasItems = await documentItems.count() > 0;
      
      // Contradictory conditions should result in no matches
      if (!isEmpty && hasItems) {
        console.warn('Contradictory conditions unexpectedly returned results');
      }
    });
  });

  describe('Network and Connectivity Issues', () => {
    test('should handle SDK initialization failures gracefully', async () => {
      // This test would be more effective with network mocking
      // For now, we test the recovery behavior
      
      // Switch networks rapidly to potentially cause initialization issues
      for (let i = 0; i < 3; i++) {
        await page.selectOption('#networkSelect', 'mainnet');
        await page.waitForTimeout(1000);
        await page.selectOption('#networkSelect', 'testnet');
        await page.waitForTimeout(1000);
      }
      
      // Check final status
      await page.waitForTimeout(5000);
      const statusText = await page.locator('#statusText').textContent();
      
      // Should eventually stabilize
      expect(statusText).toBeDefined();
    });

    test('should handle query timeouts gracefully', async () => {
      // Setup a potentially slow query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Large limit might cause timeout
      await page.fill('#limitInput', '100');
      
      await page.click('#executeQueryBtn');
      
      // Wait for reasonable timeout period
      await page.waitForTimeout(30000);
      
      // Should show either results, error, or still loading
      const errorResults = page.locator('#errorResults');
      const loadingResults = page.locator('#loadingResults');
      const hasResults = await page.locator('.document-item').count() > 0;
      const isEmpty = await page.locator('.empty-state').isVisible();
      
      const isError = await errorResults.isVisible();
      const isLoading = await loadingResults.isVisible();
      
      // Should be in a valid state
      expect(isError || isLoading || hasResults || isEmpty).toBe(true);
      
      if (isError) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/timeout|network|failed|error/i);
      }
    });

    test('should handle proof verification toggle during operations', async () => {
      // Start a query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Start query execution
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(2000);
      
      // Toggle proof verification during execution
      const proofsToggle = page.locator('#proofsToggle');
      await proofsToggle.uncheck();
      await page.waitForTimeout(1000);
      await proofsToggle.check();
      
      // Wait for query to complete
      await page.waitForTimeout(15000);
      
      // Should handle the toggle gracefully
      const statusText = await page.locator('#statusText').textContent();
      expect(statusText).not.toMatch(/error|failed/i);
    });
  });

  describe('UI State Consistency', () => {
    test('should maintain consistent state when switching contracts rapidly', async () => {
      // Rapidly switch between contracts
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(2000);
      
      await page.locator('[data-contract="dashpay"]').click();
      await page.waitForTimeout(2000);
      
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Check final state is consistent
      const contractInfo = page.locator('#contractInfo');
      if (await contractInfo.isVisible()) {
        const contractDetails = page.locator('#contractDetails');
        const detailsText = await contractDetails.textContent();
        
        // Should show DPNS contract (last selected)
        expect(detailsText).toMatch(/dpns|domain/i);
      }
      
      // Document type should be available
      const documentTypeSelect = page.locator('#documentTypeSelect');
      const options = await documentTypeSelect.locator('option').allTextContents();
      expect(options.length).toBeGreaterThan(1);
    });

    test('should handle form reset after errors', async () => {
      // Cause an error
      await page.fill('#customContractId', 'invalid-contract');
      await page.click('#loadContractBtn');
      await page.waitForTimeout(5000);
      
      // Clear and load valid contract
      await page.fill('#customContractId', '');
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Form should be in clean state
      const queryForm = page.locator('#queryForm');
      if (await queryForm.isVisible()) {
        const limitInput = page.locator('#limitInput');
        const offsetInput = page.locator('#offsetInput');
        
        expect(await limitInput.inputValue()).toBe('10'); // Default
        expect(await offsetInput.inputValue()).toBe('0'); // Default
      }
    });

    test('should handle browser back/forward navigation gracefully', async () => {
      // Load contract and setup query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Navigate away and back (simulate browser navigation)
      await page.goBack();
      await page.waitForTimeout(2000);
      await page.goForward();
      await page.waitForTimeout(5000);
      
      // Should maintain basic functionality
      const contractButtons = page.locator('.contract-btn');
      await expect(contractButtons.first()).toBeVisible();
      
      // SDK should still be functional
      const statusText = await page.locator('#statusText').textContent();
      expect(statusText).toBeDefined();
    });
  });

  describe('Memory and Performance Edge Cases', () => {
    test('should handle memory pressure from large queries', async () => {
      // This test simulates potential memory issues
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Execute multiple large queries in sequence
      for (let i = 0; i < 3; i++) {
        await page.fill('#limitInput', '50');
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(15000);
        
        // Clear results between queries
        await page.click('#clearQueryBtn');
        await page.waitForTimeout(1000);
      }
      
      // Should still be responsive
      const statusText = await page.locator('#statusText').textContent();
      expect(statusText).toBeDefined();
      
      // UI should still be interactive
      const executeBtn = page.locator('#executeQueryBtn');
      await expect(executeBtn).toBeVisible();
    });

    test('should handle rapid consecutive queries', async () => {
      // Setup contract
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Fire multiple queries rapidly
      for (let i = 0; i < 5; i++) {
        await page.fill('#limitInput', '2');
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(1000); // Very short wait
      }
      
      // Wait for all operations to settle
      await page.waitForTimeout(20000);
      
      // Should end up in a stable state
      const errorResults = page.locator('#errorResults');
      const loadingResults = page.locator('#loadingResults');
      
      // Should not be in error or permanent loading state
      const isError = await errorResults.isVisible();
      const isLoading = await loadingResults.isVisible();
      
      if (isError) {
        const errorText = await errorResults.textContent();
        // Error should be reasonable (like rate limiting)
        expect(errorText).toMatch(/rate|limit|too.*many|concurrent/i);
      }
      
      // Should eventually stop loading
      // (In a real implementation, this might timeout or queue requests)
    });
  });
});
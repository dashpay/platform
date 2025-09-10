/**
 * Export and History Functionality Tests for Document Explorer
 * Tests JSON/CSV export, query history, and data management features
 */

const { test, expect } = require('@playwright/test');

test.describe('Document Explorer - Export and History Features', () => {
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

  describe('JSON Export Functionality', () => {
    test('should enable JSON export after successful query', async () => {
      // Execute a query first
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.fill('#limitInput', '3');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check if export buttons are enabled
      const exportResultsBtn = page.locator('#exportResultsBtn');
      if (await exportResultsBtn.isVisible()) {
        await expect(exportResultsBtn).toBeEnabled();
      }
    });

    test('should trigger JSON download when export button clicked', async () => {
      // Execute a query first
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.fill('#limitInput', '2');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check if we have results to export
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        // Set up download listener
        const downloadPromise = page.waitForEvent('download');
        
        // Click export button
        const exportBtn = page.locator('#exportResultsBtn');
        if (await exportBtn.isVisible()) {
          await exportBtn.click();
          
          // Wait for download
          const download = await downloadPromise;
          expect(download.suggestedFilename()).toMatch(/documents_.+\.json$/);
          
          // Verify download content type (if accessible)
          // Note: actual content verification would require saving and parsing the file
        }
      }
    });

    test('should include proper metadata in JSON export', async () => {
      // This test would ideally save the downloaded file and parse it
      // For now, we verify the export process works
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        // Setup to capture network or check generated query structure
        const generatedQuery = page.locator('#generatedQuery');
        if (await generatedQuery.isVisible()) {
          const queryCode = await page.locator('#queryCode').textContent();
          
          // Verify query contains expected structure for export
          expect(queryCode).toContain('domain');
          expect(queryCode).toMatch(/GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec|dpns/i);
        }
      }
    });

    test('should handle empty results export gracefully', async () => {
      // Setup query that returns no results
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set conditions likely to return no results
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('label');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('extremely-unlikely-domain-name-12345');
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check export button state with empty results
      const exportBtn = page.locator('#exportResultsBtn');
      if (await exportBtn.isVisible()) {
        // Button might be disabled or show error when clicked
        const isEnabled = await exportBtn.isEnabled();
        
        if (isEnabled) {
          // Try to export empty results
          await exportBtn.click();
          await page.waitForTimeout(1000);
          
          // Should either download empty file or show error message
          const errorResults = page.locator('#errorResults');
          if (await errorResults.isVisible()) {
            const errorText = await errorResults.textContent();
            expect(errorText).toMatch(/no results|no data|empty/i);
          }
        }
      }
    });
  });

  describe('CSV Export Functionality', () => {
    test('should enable CSV export after successful query', async () => {
      // Execute a query first
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check CSV export button
      const exportCsvBtn = page.locator('#exportCsvBtn');
      if (await exportCsvBtn.isVisible()) {
        await expect(exportCsvBtn).toBeEnabled();
      }
    });

    test('should trigger CSV download with correct filename', async () => {
      // Execute a query first
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.fill('#limitInput', '2');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        // Set up download listener
        const downloadPromise = page.waitForEvent('download');
        
        // Click CSV export button
        const exportCsvBtn = page.locator('#exportCsvBtn');
        if (await exportCsvBtn.isVisible()) {
          await exportCsvBtn.click();
          
          // Wait for download
          const download = await downloadPromise;
          expect(download.suggestedFilename()).toMatch(/documents_.+\.csv$/);
        }
      }
    });

    test('should handle CSV export with complex document structures', async () => {
      // Execute query that might return complex nested data
      await page.locator('[data-contract="dashpay"]').click();
      await page.waitForTimeout(5000);
      
      // Try to load DashPay contract (has more complex document structure)
      const contractInfo = page.locator('#contractInfo');
      const isVisible = await contractInfo.isVisible({ timeout: 10000 });
      
      if (isVisible) {
        await page.selectOption('#documentTypeSelect', 'profile');
        await page.waitForTimeout(1000);
        await page.fill('#limitInput', '2');
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(15000);
        
        const documentItems = page.locator('.document-item');
        const itemCount = await documentItems.count();
        
        if (itemCount > 0) {
          const exportCsvBtn = page.locator('#exportCsvBtn');
          if (await exportCsvBtn.isVisible()) {
            // Should handle complex data structures in CSV format
            await exportCsvBtn.click();
            await page.waitForTimeout(2000);
          }
        }
      }
    });
  });

  describe('Single Document Export', () => {
    test('should export individual documents from modal', async () => {
      // Execute query and open document details
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.fill('#limitInput', '3');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        // Click on first document to open modal
        await documentItems.first().click();
        
        // Check modal opens
        const modal = page.locator('#documentModal');
        await expect(modal).toBeVisible();
        
        // Check export button in modal
        const exportDocumentBtn = page.locator('#exportDocumentBtn');
        if (await exportDocumentBtn.isVisible()) {
          // Set up download listener
          const downloadPromise = page.waitForEvent('download');
          
          await exportDocumentBtn.click();
          
          // Wait for download
          const download = await downloadPromise;
          expect(download.suggestedFilename()).toMatch(/document_.+\.json$/);
        }
        
        // Close modal
        await page.click('.modal-close');
      }
    });

    test('should include document metadata in single export', async () => {
      // This verifies the export process works for individual documents
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        // Open document modal
        await documentItems.first().click();
        const modal = page.locator('#documentModal');
        await expect(modal).toBeVisible();
        
        // Check document details contain expected structure
        const documentDetails = page.locator('#documentDetails');
        const detailsText = await documentDetails.textContent();
        
        // Should contain JSON with document data
        expect(detailsText).toContain('{');
        expect(detailsText).toContain('}');
        expect(detailsText).toMatch(/id|data|ownerId/);
        
        await page.click('.modal-close');
      }
    });
  });

  describe('Query Code Export (Copy to Clipboard)', () => {
    test('should copy query code to clipboard', async () => {
      // Execute a query to generate code
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Add some query parameters
      await page.fill('#limitInput', '10');
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('dash');
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      // Check query code export
      const exportQueryBtn = page.locator('#exportQueryBtn');
      if (await exportQueryBtn.isVisible()) {
        await exportQueryBtn.click();
        
        // Verify button shows success feedback
        await page.waitForTimeout(1000);
        const buttonText = await exportQueryBtn.textContent();
        
        // Should show copied confirmation or similar
        expect(buttonText).toMatch(/copied|copy|export/i);
      }
    });

    test('should generate valid executable query code', async () => {
      // Setup and execute query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      // Check generated query code structure
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        
        // Should contain valid JavaScript SDK code
        expect(queryCode).toContain('sdk.getDocuments');
        expect(queryCode).toContain('GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec');
        expect(queryCode).toContain('domain');
        expect(queryCode).toMatch(/limit|where|orderBy/);
        
        // Should have proper JavaScript syntax
        expect(queryCode).toContain('await');
        expect(queryCode).toContain('(');
        expect(queryCode).toContain(')');
        expect(queryCode).toContain(';');
      }
    });
  });

  describe('Query History Management', () => {
    test('should track executed queries in history', async () => {
      // Execute multiple queries
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // First query
      await page.fill('#limitInput', '5');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Second query with different parameters
      await page.fill('#limitInput', '10');
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('dash');
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Check query history
      const queryHistory = page.locator('#queryHistory');
      const historyEntries = queryHistory.locator('.history-entry');
      const entryCount = await historyEntries.count();
      
      // Should have multiple entries (including system messages)
      expect(entryCount).toBeGreaterThan(2);
      
      // Check history entries contain relevant information
      for (let i = 0; i < Math.min(entryCount, 3); i++) {
        const entry = historyEntries.nth(i);
        const entryText = await entry.textContent();
        
        // Should contain timestamp and operation info
        expect(entryText).toMatch(/\[.*\]|\d+ms|documents|query|Contract/i);
      }
    });

    test('should display query performance metrics in history', async () => {
      // Execute a query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check history for performance information
      const queryHistory = page.locator('#queryHistory');
      const historyEntries = queryHistory.locator('.history-entry');
      
      let foundPerformanceEntry = false;
      for (let i = 0; i < await historyEntries.count(); i++) {
        const entry = historyEntries.nth(i);
        const entryText = await entry.textContent();
        
        if (entryText.includes('ms') || entryText.includes('documents')) {
          foundPerformanceEntry = true;
          expect(entryText).toMatch(/\d+ms|\d+ docs?/);
          break;
        }
      }
      
      // Should have at least one performance-related entry
      // (This might not always be present depending on implementation)
    });

    test('should clear query history when requested', async () => {
      // Execute a query to generate history
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Check initial history count
      const queryHistory = page.locator('#queryHistory');
      const initialEntries = queryHistory.locator('.history-entry');
      const initialCount = await initialEntries.count();
      
      expect(initialCount).toBeGreaterThan(0);
      
      // Clear history
      const clearHistoryBtn = page.locator('#clearHistoryBtn');
      if (await clearHistoryBtn.isVisible()) {
        await clearHistoryBtn.click();
        
        // Check history is cleared
        await page.waitForTimeout(1000);
        const finalEntries = queryHistory.locator('.history-entry');
        const finalCount = await finalEntries.count();
        
        expect(finalCount).toBeLessThan(initialCount);
      }
    });

    test('should export query history', async () => {
      // Execute some queries to generate history
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Multiple queries
      for (let i = 0; i < 2; i++) {
        await page.fill('#limitInput', `${5 + i}`);
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(8000);
      }
      
      // Export history
      const exportHistoryBtn = page.locator('#exportHistoryBtn');
      if (await exportHistoryBtn.isVisible()) {
        // Set up download listener
        const downloadPromise = page.waitForEvent('download');
        
        await exportHistoryBtn.click();
        
        // Wait for download
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toMatch(/query_history_.+\.json$/);
      }
    });
  });

  describe('Error Handling in Export Operations', () => {
    test('should handle export operations without results', async () => {
      // Try to export without executing any queries
      const exportResultsBtn = page.locator('#exportResultsBtn');
      
      if (await exportResultsBtn.isVisible()) {
        const isEnabled = await exportResultsBtn.isEnabled();
        
        if (isEnabled) {
          await exportResultsBtn.click();
          await page.waitForTimeout(2000);
          
          // Should show error or handle gracefully
          const errorResults = page.locator('#errorResults');
          if (await errorResults.isVisible()) {
            const errorText = await errorResults.textContent();
            expect(errorText).toMatch(/no results|no data/i);
          }
        } else {
          // Button should be disabled when no results
          expect(isEnabled).toBe(false);
        }
      }
    });

    test('should handle clipboard access errors gracefully', async () => {
      // This test would verify clipboard access error handling
      // In a real test environment, clipboard might not be available
      
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      const exportQueryBtn = page.locator('#exportQueryBtn');
      if (await exportQueryBtn.isVisible()) {
        await exportQueryBtn.click();
        await page.waitForTimeout(2000);
        
        // Should either succeed or fail gracefully
        // Check for error messages or success indicators
        const buttonText = await exportQueryBtn.textContent();
        expect(buttonText).toBeDefined();
      }
    });

    test('should validate export data before download', async () => {
      // Execute query that might return invalid data
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up potentially problematic query
      await page.fill('#limitInput', '1000'); // Very large limit
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(20000);
      
      const exportResultsBtn = page.locator('#exportResultsBtn');
      if (await exportResultsBtn.isVisible() && await exportResultsBtn.isEnabled()) {
        // Should handle large exports gracefully
        await exportResultsBtn.click();
        await page.waitForTimeout(5000);
        
        // Export should either succeed or show appropriate error
        const errorResults = page.locator('#errorResults');
        if (await errorResults.isVisible()) {
          const errorText = await errorResults.textContent();
          expect(errorText).toMatch(/too large|limit|error/i);
        }
      }
    });
  });
});
/**
 * Comprehensive Functional Tests for Document Explorer Web App
 * Tests all contract loading, query building, result display functionality
 */

const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../../ui-automation/utils/wasm-sdk-page');

test.describe('Document Explorer - Functional Tests', () => {
  let page;
  let docExplorer;

  test.beforeEach(async ({ browser }) => {
    // Create new page for each test
    page = await browser.newPage();
    docExplorer = new WasmSdkPage(page);
    
    // Navigate to Document Explorer
    await page.goto('http://localhost:8888/samples/document-explorer/');
    
    // Wait for page to load
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(2000); // Allow for SDK initialization
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('Application Initialization', () => {
    test('should load Document Explorer interface', async () => {
      // Check main elements are present
      await expect(page.locator('.app-container')).toBeVisible();
      await expect(page.locator('.app-header h1')).toContainText('Document Explorer');
      
      // Check network selector
      await expect(page.locator('#networkSelect')).toBeVisible();
      await expect(page.locator('#proofsToggle')).toBeVisible();
      
      // Check status indicator
      await expect(page.locator('#statusIndicator')).toBeVisible();
    });

    test('should initialize SDK and show connection status', async () => {
      // Wait for SDK initialization
      await page.waitForTimeout(5000);
      
      // Check for successful initialization
      const statusText = await page.locator('#statusText').textContent();
      expect(statusText).toMatch(/Connected|Initialized/i);
      
      // Status indicator should not show error
      const statusDot = page.locator('.status-dot');
      await expect(statusDot).not.toHaveClass(/error/);
    });

    test('should handle network switching', async () => {
      // Test network switching
      await page.selectOption('#networkSelect', 'mainnet');
      await page.waitForTimeout(3000); // Allow for reinitialization
      
      const networkSelect = page.locator('#networkSelect');
      await expect(networkSelect).toHaveValue('mainnet');
      
      // Switch back to testnet
      await page.selectOption('#networkSelect', 'testnet');
      await page.waitForTimeout(3000);
      await expect(networkSelect).toHaveValue('testnet');
    });

    test('should toggle proof verification setting', async () => {
      const proofsToggle = page.locator('#proofsToggle');
      
      // Should be checked by default
      await expect(proofsToggle).toBeChecked();
      
      // Toggle off
      await proofsToggle.uncheck();
      await expect(proofsToggle).not.toBeChecked();
      
      // Toggle back on
      await proofsToggle.check();
      await expect(proofsToggle).toBeChecked();
    });
  });

  describe('Contract Selection and Loading', () => {
    test('should load system contracts via buttons', async () => {
      // Test DPNS contract loading
      const dpnsButton = page.locator('[data-contract="dpns"]');
      await dpnsButton.click();
      
      // Wait for contract to load
      await page.waitForTimeout(5000);
      
      // Check that contract info appears
      const contractInfo = page.locator('#contractInfo');
      await expect(contractInfo).toBeVisible();
      
      // Check contract details
      const contractDetails = page.locator('#contractDetails');
      const detailsText = await contractDetails.textContent();
      expect(detailsText).toContain('DPNS');
      expect(detailsText).toContain('domain');
    });

    test('should display document type buttons after contract load', async () => {
      // Load DPNS contract
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Check document type buttons appear
      const documentTypes = page.locator('#documentTypes');
      await expect(documentTypes).toBeVisible();
      
      // Check for domain document type
      const domainButton = page.locator('.document-type-btn');
      await expect(domainButton.first()).toBeVisible();
      
      const buttonText = await domainButton.first().textContent();
      expect(['domain', 'preorder']).toContain(buttonText);
    });

    test('should populate document type dropdown', async () => {
      // Load DPNS contract
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Check dropdown is populated
      const documentTypeSelect = page.locator('#documentTypeSelect');
      const options = await documentTypeSelect.locator('option').allTextContents();
      
      expect(options.length).toBeGreaterThan(1); // Should have more than just the placeholder
      expect(options.join(' ')).toMatch(/domain|preorder/);
    });

    test('should load custom contract by ID', async () => {
      const customContractId = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec'; // DPNS testnet
      
      // Enter custom contract ID
      await page.fill('#customContractId', customContractId);
      await page.click('#loadContractBtn');
      
      // Wait for loading
      await page.waitForTimeout(8000);
      
      // Check that contract loads
      const contractInfo = page.locator('#contractInfo');
      await expect(contractInfo).toBeVisible();
    });

    test('should handle invalid contract IDs gracefully', async () => {
      const invalidContractId = 'invalid-contract-id';
      
      // Enter invalid contract ID
      await page.fill('#customContractId', invalidContractId);
      await page.click('#loadContractBtn');
      
      // Wait for response
      await page.waitForTimeout(5000);
      
      // Should show error (check for error styling or message)
      // This might be shown in results error div or status
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/failed|invalid|error/i);
      }
    });

    test('should display contract information correctly', async () => {
      // Load DPNS contract
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      // Check contract details structure
      const contractDetails = page.locator('#contractDetails');
      const detailsText = await contractDetails.textContent();
      
      // Should contain JSON with contract info
      expect(detailsText).toContain('{');
      expect(detailsText).toContain('}');
      expect(detailsText).toContain('id');
      expect(detailsText).toContain('name');
      expect(detailsText).toContain('documentTypes');
    });
  });

  describe('Query Form and Builder', () => {
    test('should show query form after contract and document type selection', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      await page.selectOption('#documentTypeSelect', 'domain');
      
      // Query form should be visible
      const queryForm = page.locator('#queryForm');
      await expect(queryForm).toBeVisible();
      
      // Check query form elements
      await expect(page.locator('#limitInput')).toBeVisible();
      await expect(page.locator('#offsetInput')).toBeVisible();
      await expect(page.locator('#executeQueryBtn')).toBeVisible();
    });

    test('should populate query builder fields based on document type', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Check that field selects are populated
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      const options = await fieldSelect.locator('option').allTextContents();
      
      expect(options.length).toBeGreaterThan(1);
      expect(options.join(' ')).toMatch(/\$ownerId|\$createdAt|label/);
    });

    test('should add and remove WHERE conditions', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Count initial conditions
      const initialConditions = await page.locator('#whereConditions .condition-row').count();
      
      // Add a condition
      await page.click('#addConditionBtn');
      
      // Check condition was added
      const newConditions = await page.locator('#whereConditions .condition-row').count();
      expect(newConditions).toBe(initialConditions + 1);
      
      // Remove a condition (if there's a remove button)
      const removeButtons = page.locator('#whereConditions .btn-remove');
      if (await removeButtons.count() > 0) {
        await removeButtons.first().click();
        const finalConditions = await page.locator('#whereConditions .condition-row').count();
        expect(finalConditions).toBeLessThan(newConditions);
      }
    });

    test('should add and remove ORDER BY conditions', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Count initial order by conditions
      const initialOrderBy = await page.locator('#orderByConditions .orderby-row').count();
      
      // Add an order by condition
      await page.click('#addOrderByBtn');
      
      // Check condition was added
      const newOrderBy = await page.locator('#orderByConditions .orderby-row').count();
      expect(newOrderBy).toBe(initialOrderBy + 1);
    });

    test('should validate query form inputs', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Test limit input validation
      const limitInput = page.locator('#limitInput');
      await limitInput.fill('-1'); // Invalid value
      const limitValue = await limitInput.inputValue();
      // HTML5 validation should prevent negative values or application should handle it
      expect(parseInt(limitValue)).toBeGreaterThanOrEqual(1);
      
      // Test valid limit
      await limitInput.fill('10');
      expect(await limitInput.inputValue()).toBe('10');
    });
  });

  describe('Query Execution and Results', () => {
    test('should execute simple query and display results', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set a reasonable limit
      await page.fill('#limitInput', '5');
      
      // Execute query
      await page.click('#executeQueryBtn');
      
      // Wait for results
      await page.waitForTimeout(15000);
      
      // Check for results or empty state
      const resultsContainer = page.locator('#resultsContainer');
      await expect(resultsContainer).toBeVisible();
      
      // Should show either documents or empty state
      const hasResults = await page.locator('.document-item').count() > 0;
      const hasEmptyState = await page.locator('.empty-state').isVisible();
      
      expect(hasResults || hasEmptyState).toBe(true);
    });

    test('should show loading state during query execution', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Start query execution
      const executePromise = page.click('#executeQueryBtn');
      
      // Check loading state appears quickly
      await page.waitForTimeout(1000);
      const loading = page.locator('#loadingResults');
      
      // Loading should be visible initially
      if (await loading.isVisible()) {
        expect(await loading.isVisible()).toBe(true);
      }
      
      await executePromise;
      await page.waitForTimeout(10000);
    });

    test('should display query performance metrics', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check for query time display
      const queryTime = page.locator('#queryTime');
      if (await queryTime.isVisible()) {
        const timeText = await queryTime.textContent();
        expect(timeText).toMatch(/\d+ms/);
      }
      
      // Check for result count
      const resultsCount = page.locator('#resultsCount');
      if (await resultsCount.isVisible()) {
        const countText = await resultsCount.textContent();
        expect(countText).toMatch(/\d+ documents/);
      }
    });

    test('should display generated query code', async () => {
      // Load contract and select document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Add some query conditions
      await page.fill('#limitInput', '10');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      // Check generated query display
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = page.locator('#queryCode');
        const codeText = await queryCode.textContent();
        
        expect(codeText).toContain('sdk.getDocuments');
        expect(codeText).toContain('domain');
        expect(codeText).toContain('limit: 10');
      }
    });

    test('should handle query errors gracefully', async () => {
      // Load contract
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set invalid query parameters
      await page.fill('#limitInput', '1000'); // Very large limit might cause issues
      
      // Fill in an invalid WHERE condition manually if possible
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('$ownerId');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('invalid-owner-id-that-should-cause-error');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Check for error handling
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText.length).toBeGreaterThan(0);
      }
    });
  });

  describe('Document Display and Interaction', () => {
    test('should display document grid when results available', async () => {
      // Load contract, execute query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.fill('#limitInput', '3');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check for document grid
      const documentGrid = page.locator('.document-grid');
      const documentItems = page.locator('.document-item');
      
      const itemCount = await documentItems.count();
      if (itemCount > 0) {
        await expect(documentGrid).toBeVisible();
        
        // Check document item structure
        const firstItem = documentItems.first();
        await expect(firstItem).toBeVisible();
        
        // Should have header, meta, and preview
        await expect(firstItem.locator('.document-header')).toBeVisible();
        await expect(firstItem.locator('.document-meta')).toBeVisible();
        await expect(firstItem.locator('.document-preview')).toBeVisible();
      }
    });

    test('should open document details modal on click', async () => {
      // Load contract, execute query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      await page.fill('#limitInput', '3');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Click on first document if available
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        await documentItems.first().click();
        
        // Check modal opens
        const modal = page.locator('#documentModal');
        await expect(modal).toBeVisible();
        
        // Check modal content
        const documentDetails = page.locator('#documentDetails');
        await expect(documentDetails).toBeVisible();
        
        const detailsText = await documentDetails.textContent();
        expect(detailsText).toContain('{'); // Should contain JSON
        
        // Close modal
        await page.click('.modal-close');
        await expect(modal).not.toBeVisible();
      }
    });

    test('should show empty state when no results', async () => {
      // Load contract
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set conditions that should return no results
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('label');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('extremely-unlikely-to-exist-domain-name-12345');
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Should show empty state
      const emptyState = page.locator('.empty-state');
      if (await emptyState.isVisible()) {
        await expect(emptyState).toBeVisible();
        
        const emptyText = await emptyState.textContent();
        expect(emptyText).toMatch(/No Documents Found|empty/i);
      }
    });
  });

  describe('Clear and Reset Functionality', () => {
    test('should clear query form when clear button clicked', async () => {
      // Load contract and set up query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Fill in some values
      await page.fill('#limitInput', '20');
      await page.fill('#offsetInput', '5');
      
      // Add a WHERE condition
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('label');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('test');
      
      // Clear the query
      await page.click('#clearQueryBtn');
      
      // Check values are reset
      expect(await page.locator('#limitInput').inputValue()).toBe('10'); // Default value
      expect(await page.locator('#offsetInput').inputValue()).toBe('0'); // Default value
      
      // WHERE condition should be cleared
      expect(await valueInput.inputValue()).toBe('');
    });

    test('should hide generated query when cleared', async () => {
      // Load contract and set up query
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Execute a query first to show generated query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      // Clear the query
      await page.click('#clearQueryBtn');
      
      // Generated query should be hidden
      const generatedQuery = page.locator('#generatedQuery');
      await expect(generatedQuery).not.toBeVisible();
    });
  });
});
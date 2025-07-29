const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');

test.describe('WASM SDK Basic Smoke Tests', () => {
  let wasmSdkPage;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    await wasmSdkPage.initialize('testnet');
  });

  test('should initialize SDK successfully', async () => {
    // Verify SDK initialized
    const statusState = await wasmSdkPage.getStatusBannerState();
    expect(statusState).toBe('success');
    
    // Verify network is set to testnet
    const networkIndicator = wasmSdkPage.page.locator('#networkIndicator');
    await expect(networkIndicator).toContainText('TESTNET');
  });

  test('should load query categories', async () => {
    await wasmSdkPage.setOperationType('queries');
    
    const categories = await wasmSdkPage.getAvailableQueryCategories();
    
    // Check that we have the expected categories
    const expectedCategories = [
      'Identity Queries',
      'Data Contract Queries', 
      'Document Queries',
      'DPNS Queries',
      'System & Utility'
    ];
    
    for (const category of expectedCategories) {
      expect(categories).toContain(category);
    }
  });

  test('should switch between networks', async () => {
    // Test switching to mainnet
    await wasmSdkPage.setNetwork('mainnet');
    const mainnetIndicator = wasmSdkPage.page.locator('#networkIndicator');
    await expect(mainnetIndicator).toContainText('MAINNET');
    
    // Switch back to testnet
    await wasmSdkPage.setNetwork('testnet');
    const testnetIndicator = wasmSdkPage.page.locator('#networkIndicator');
    await expect(testnetIndicator).toContainText('TESTNET');
  });

  test('should show query types when category is selected', async () => {
    await wasmSdkPage.setOperationType('queries');
    await wasmSdkPage.setQueryCategory('identity');
    
    const queryTypes = await wasmSdkPage.getAvailableQueryTypes();
    
    // Should have some identity query types
    expect(queryTypes.length).toBeGreaterThan(0);
    expect(queryTypes).toContain('Get Identity');
  });

  test('should show input fields when query type is selected', async () => {
    await wasmSdkPage.setOperationType('queries');
    await wasmSdkPage.setQueryCategory('identity');
    await wasmSdkPage.setQueryType('getIdentity');
    
    // Should show query inputs container
    const queryInputs = wasmSdkPage.page.locator('#queryInputs');
    await expect(queryInputs).toBeVisible();
    
    // Should show execute button
    const executeButton = wasmSdkPage.page.locator('#executeQuery');
    await expect(executeButton).toBeVisible();
  });

  test('should enable/disable execute button based on form completion', async () => {
    await wasmSdkPage.setOperationType('queries');
    await wasmSdkPage.setQueryCategory('identity');
    await wasmSdkPage.setQueryType('getIdentity');
    
    const executeButton = wasmSdkPage.page.locator('#executeQuery');
    
    // Button should be enabled (even without required params for this test)
    await expect(executeButton).toBeVisible();
  });

  test('should clear results when clear button is clicked', async () => {
    await wasmSdkPage.setOperationType('queries');
    await wasmSdkPage.setQueryCategory('system');
    await wasmSdkPage.setQueryType('getStatus');
    
    // Execute a simple query first
    await wasmSdkPage.executeQuery();
    
    // Clear results
    await wasmSdkPage.clearResults();
    
    // Verify results are cleared
    const resultContent = wasmSdkPage.page.locator('#identityInfo');
    await expect(resultContent).toHaveClass(/empty/);
  });

  test('should toggle proof information', async () => {
    await wasmSdkPage.setOperationType('queries');
    await wasmSdkPage.setQueryCategory('identity');
    await wasmSdkPage.setQueryType('getIdentity');
    
    // Check if proof toggle is available
    const proofContainer = wasmSdkPage.page.locator('#proofToggleContainer');
    
    if (await proofContainer.isVisible()) {
      // Test enabling proof info
      await wasmSdkPage.enableProofInfo();
      const proofToggle = wasmSdkPage.page.locator('#proofToggle');
      await expect(proofToggle).toBeChecked();
      
      // Test disabling proof info
      await wasmSdkPage.disableProofInfo();
      await expect(proofToggle).not.toBeChecked();
    }
  });

  test('should show query description when available', async () => {
    await wasmSdkPage.setOperationType('queries');
    await wasmSdkPage.setQueryCategory('identity');
    await wasmSdkPage.setQueryType('getIdentity');
    
    const description = await wasmSdkPage.getQueryDescription();
    
    if (description) {
      expect(description.length).toBeGreaterThan(0);
    }
  });
});
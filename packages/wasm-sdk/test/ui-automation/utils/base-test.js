const { expect } = require('@playwright/test');

/**
 * Base test utilities for WASM SDK UI automation
 */
class BaseTest {
  constructor(page) {
    this.page = page;
  }

  /**
   * Navigate to the WASM SDK index page and wait for initialization
   */
  async navigateToSdk() {
    await this.page.goto('/');
    
    // Wait for the WASM SDK to initialize
    await this.page.waitForSelector('#statusBanner.success', { 
      timeout: 60000,
      state: 'visible' 
    });
    
    // Verify we're on the right page
    await expect(this.page).toHaveTitle(/Dash Platform WASM JS SDK/);
    
    console.log('✅ SDK initialized successfully');
  }

  /**
   * Wait for network loading to complete
   */
  async waitForNetworkIdle() {
    await this.page.waitForLoadState('networkidle');
  }

  /**
   * Take a screenshot with a descriptive name
   */
  async takeScreenshot(name) {
    await this.page.screenshot({ 
      path: `test-results/screenshots/${name}-${Date.now()}.png`,
      fullPage: true 
    });
  }

  /**
   * Wait for an element to be visible and ready for interaction
   */
  async waitForElement(selector, options = {}) {
    const element = this.page.locator(selector);
    await element.waitFor({ state: 'visible', ...options });
    return element;
  }

  /**
   * Fill input field with validation
   */
  async fillInput(selector, value, options = {}) {
    const input = await this.waitForElement(selector);
    await input.clear();
    await input.fill(value);
    
    // Verify the value was set correctly
    if (options.verify !== false) {
      await expect(input).toHaveValue(value);
    }
    
    return input;
  }

  /**
   * Select option from dropdown
   */
  async selectOption(selector, value) {
    const select = await this.waitForElement(selector);
    await select.selectOption(value);
    
    // Verify selection
    await expect(select).toHaveValue(value);
    
    return select;
  }

  /**
   * Click button and wait for any loading states
   */
  async clickButton(selector, options = {}) {
    const button = await this.waitForElement(selector);
    
    // Check if button is enabled
    await expect(button).toBeEnabled();
    
    // Click and optionally wait for response
    await button.click();
    
    if (options.waitForResponse) {
      await this.page.waitForResponse(response => 
        response.url().includes('dapi') || response.status() !== 200
      );
    }
    
    return button;
  }

  /**
   * Get the current result content
   */
  async getResultContent() {
    const resultContainer = this.page.locator('#identityInfo');
    await resultContainer.waitFor({ state: 'visible' });
    return await resultContainer.textContent();
  }

  /**
   * Check if result shows an error
   */
  async hasErrorResult() {
    const resultContainer = this.page.locator('#identityInfo');
    const classList = await resultContainer.getAttribute('class');
    return classList && classList.includes('error');
  }

  /**
   * Clear results
   */
  async clearResults() {
    await this.clickButton('#clearButton');
    const resultContainer = this.page.locator('#identityInfo');
    await expect(resultContainer).toHaveClass(/empty/);
  }

  /**
   * Set network (mainnet/testnet)
   */
  async setNetwork(network = 'testnet') {
    const networkRadio = this.page.locator(`#${network}`);
    await networkRadio.check();
    
    // Wait for network indicator to update
    const indicator = this.page.locator('#networkIndicator');
    await expect(indicator).toContainText(network.toUpperCase());
    
    console.log(`✅ Network set to ${network}`);
  }

  /**
   * Set operation type (queries, transitions, wallet)
   */
  async setOperationType(type = 'queries') {
    await this.selectOption('#operationType', type);
    console.log(`✅ Operation type set to ${type}`);
  }

  /**
   * Set query category
   */
  async setQueryCategory(category) {
    await this.selectOption('#queryCategory', category);
    
    // Wait for query type dropdown to populate
    await this.page.waitForTimeout(500);
    
    console.log(`✅ Query category set to ${category}`);
  }

  /**
   * Set specific query type
   */
  async setQueryType(queryType) {
    // Make sure query type dropdown is visible
    await this.waitForElement('#queryType');
    await this.selectOption('#queryType', queryType);
    
    // Wait for inputs to appear
    await this.page.waitForTimeout(500);
    
    console.log(`✅ Query type set to ${queryType}`);
  }

  /**
   * Execute the current query and wait for results
   */
  async executeQuery() {
    const executeButton = this.page.locator('#executeQuery');
    
    // Ensure button is visible and enabled
    await expect(executeButton).toBeVisible();
    await expect(executeButton).toBeEnabled();
    
    // Click execute button
    await executeButton.click();
    
    // Wait for status banner to show loading
    await this.page.locator('#statusBanner.loading').waitFor({ state: 'visible' });
    
    // Wait for loading to complete (either success or error)
    await this.page.locator('#statusBanner.loading').waitFor({ state: 'hidden', timeout: 30000 });
    
    console.log('✅ Query executed');
    
    // Return whether it was successful
    const statusBanner = this.page.locator('#statusBanner');
    const statusClass = await statusBanner.getAttribute('class');
    return statusClass && statusClass.includes('success');
  }
}

module.exports = { BaseTest };
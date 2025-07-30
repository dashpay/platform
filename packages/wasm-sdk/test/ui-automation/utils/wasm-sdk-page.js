const { BaseTest } = require('./base-test');

/**
 * Page Object Model for WASM SDK index.html interface
 */
class WasmSdkPage extends BaseTest {
  constructor(page) {
    super(page);
    
    // Define selectors for all interface elements
    this.selectors = {
      // Status and initialization
      statusBanner: '#statusBanner',
      statusBannerSuccess: '#statusBanner.success',
      statusBannerLoading: '#statusBanner.loading',
      statusBannerError: '#statusBanner.error',
      
      // Network controls
      mainnetRadio: '#mainnet',
      testnetRadio: '#testnet',
      networkIndicator: '#networkIndicator',
      trustedModeCheckbox: '#trustedMode',
      
      // Operation selectors
      operationType: '#operationType',
      queryCategory: '#queryCategory',
      queryType: '#queryType',
      
      // Query inputs
      queryInputs: '#queryInputs',
      queryTitle: '#queryTitle',
      dynamicInputs: '#dynamicInputs',
      
      // Authentication inputs
      authenticationInputs: '#authenticationInputs',
      identityId: '#identityId',
      privateKey: '#privateKey',
      assetLockProof: '#assetLockProof',
      
      // Proof toggle
      proofToggleContainer: '#proofToggleContainer',
      proofToggle: '#proofToggle',
      
      // Execute button
      executeQuery: '#executeQuery',
      
      // Results
      resultContainer: '.result-container',
      resultContent: '#identityInfo',
      resultHeader: '.result-header',
      
      // Action buttons
      clearButton: '#clearButton',
      copyButton: '#copyButton',
      clearCacheButton: '#clearCacheButton',
      
      // Advanced SDK configuration
      sdkConfigDetails: '.sdk-config',
      platformVersion: '#platformVersion',
      connectTimeout: '#connectTimeout',
      requestTimeout: '#requestTimeout',
      retries: '#retries',
      banFailedAddress: '#banFailedAddress',
      applyConfig: '#applyConfig'
    };
  }

  /**
   * Initialize the SDK page
   */
  async initialize(network = 'testnet') {
    await this.navigateToSdk();
    await this.setNetwork(network);
    
    // Wait for SDK to be ready after network change
    await this.waitForSdkReady();
    
    return this;
  }

  /**
   * Set up a query test scenario
   */
  async setupQuery(category, queryType, parameters = {}) {
    // Set operation type to queries
    await this.setOperationType('queries');
    
    // Set category and query type
    await this.setQueryCategory(category);
    await this.setQueryType(queryType);
    
    // Fill in parameters
    if (Object.keys(parameters).length > 0) {
      await this.fillQueryParameters(parameters);
    }
    
    return this;
  }

  /**
   * Fill query parameters dynamically based on the input structure
   */
  async fillQueryParameters(parameters) {
    for (const [key, value] of Object.entries(parameters)) {
      await this.fillParameterByName(key, value);
    }
  }

  /**
   * Fill a specific parameter by name
   */
  async fillParameterByName(paramName, value) {
    // Special handling for array parameters that use dynamic input fields
    if (paramName === 'ids') {
      const enterValueInput = this.page.locator('input[placeholder="Enter value"]').first();
      const count = await enterValueInput.count();
      
      if (count > 0 && await enterValueInput.isVisible()) {
        await this.fillInputByType(enterValueInput, value);
        return;
      }
    }
    
    const inputSelector = `input[name="${paramName}"], select[name="${paramName}"], textarea[name="${paramName}"]`;
    const input = this.page.locator(inputSelector).first();
    
    // Check if input exists
    if (await input.count() === 0) {
      // Try alternative selectors based on common patterns
      const alternativeSelectors = [
        `#${paramName}`,
        `[id*="${paramName}"]`,
        `[placeholder*="${paramName}"]`,
        `label:has-text("${paramName}") + input`,
        `label:has-text("${paramName}") + select`,
        `label:has-text("${paramName}") + textarea`
      ];
      
      let found = false;
      for (const selector of alternativeSelectors) {
        const altInput = this.page.locator(selector).first();
        if (await altInput.count() > 0) {
          await this.fillInputByType(altInput, value);
          found = true;
          break;
        }
      }
      
      if (!found) {
        console.warn(`⚠️  Could not find input for parameter: ${paramName}`);
      }
    } else {
      await this.fillInputByType(input, value);
    }
  }

  /**
   * Fill input based on its type
   */
  async fillInputByType(inputElement, value) {
    const tagName = await inputElement.evaluate(el => el.tagName.toLowerCase());
    const inputType = await inputElement.evaluate(el => el.type);
    
    if (tagName === 'select') {
      await inputElement.selectOption(value.toString());
    } else if (inputType === 'checkbox') {
      if (value) {
        await inputElement.check();
      } else {
        await inputElement.uncheck();
      }
    } else if (Array.isArray(value)) {
      // Handle array inputs - check if there's an "Add items" button nearby
      const success = await this.handleArrayInput(inputElement, value);
      if (!success) {
        // Fallback to JSON string if array handling fails
        await inputElement.fill(JSON.stringify(value));
      }
    } else if (typeof value === 'object') {
      // Handle object inputs (JSON)
      await inputElement.fill(JSON.stringify(value));
    } else {
      // Handle text/number inputs
      await inputElement.fill(value.toString());
    }
  }

  /**
   * Handle array inputs with "Add items" button functionality
   */
  async handleArrayInput(baseElement, arrayValues) {
    try {
      // Look for existing input fields first (prioritize array container inputs)
      const arrayContainerInputs = this.page.locator('.array-input-container input[type="text"]');
      const allInputs = this.page.locator('input[type="text"], textarea').filter({
        hasNot: this.page.locator('[readonly]')
      });
      
      // Use array container inputs if available, otherwise use all inputs
      const existingInputs = await arrayContainerInputs.count() > 0 ? arrayContainerInputs : allInputs;
      const existingCount = await existingInputs.count();

      // Fill the first existing field if available
      if (existingCount > 0 && arrayValues.length > 0) {
        const firstInput = existingInputs.first();
        await firstInput.fill(arrayValues[0].toString());
      }

      // Look for "Add Item" button (specific to WASM SDK array inputs)
      const addButton = this.page.locator('button:has-text("+ Add Item"), button.add-array-item, button:has-text("Add Item"), button:has-text("Add"), button:has-text("add")').first();
      
      if (await addButton.count() === 0) {
        if (arrayValues.length <= 1) {
          return true;
        } else {
          return false;
        }
      }

      // Add remaining items (starting from index 1)
      for (let i = 1; i < arrayValues.length; i++) {
        const value = arrayValues[i];
        
        // Click "Add items" button to create new field
        await addButton.click();
        await this.page.waitForTimeout(500); // Wait for new input to appear
        
        // Find all input fields again (should be one more now)
        const currentArrayInputs = this.page.locator('.array-input-container input[type="text"]');
        const currentAllInputs = this.page.locator('input[type="text"], textarea').filter({
          hasNot: this.page.locator('[readonly]')
        });
        
        // Use array container inputs if available
        const currentInputs = await currentArrayInputs.count() > 0 ? currentArrayInputs : currentAllInputs;
        const currentCount = await currentInputs.count();
        
        if (currentCount > existingCount + (i - 1)) {
          // Fill the newest input field
          const newInput = currentInputs.nth(currentCount - 1);
          await newInput.fill(value.toString());
        } else {
          console.warn(`Could not find new input field for item ${i + 1}`);
        }
      }

      return true;
    } catch (error) {
      console.warn(`Array input handling failed: ${error.message}`);
      return false;
    }
  }

  /**
   * Enable proof information toggle
   */
  async enableProofInfo() {
    // Wait a moment for the UI to fully load after query setup
    await this.page.waitForTimeout(1000);
    
    const proofContainer = this.page.locator(this.selectors.proofToggleContainer);
    
    // Check if proof container exists and becomes visible
    try {
      // Wait longer and check if container becomes visible or is already attached
      await proofContainer.waitFor({ state: 'attached', timeout: 10000 });
      
      // Check if it's visible or can be made visible
      const isVisible = await proofContainer.isVisible();
      if (!isVisible) {
        // It might be hidden by display:none, check if it exists in the DOM
        const count = await proofContainer.count();
        if (count === 0) {
          console.log('⚠️ Proof toggle container not found in DOM');
          return false;
        }
        
        // Try to wait a bit more for it to become visible
        try {
          await proofContainer.waitFor({ state: 'visible', timeout: 3000 });
        } catch {
          console.log('⚠️ Proof toggle container exists but remains hidden - may not be available for this query type');
          return false;
        }
      }
      
      const proofToggle = this.page.locator(this.selectors.proofToggle);
      await proofToggle.waitFor({ state: 'visible', timeout: 3000 });
      
      // Check if already enabled
      const isChecked = await proofToggle.isChecked();
      if (!isChecked) {
        await proofToggle.check();
      }
      
      console.log('✅ Proof info enabled');
      return true;
    } catch (error) {
      console.log(`⚠️ Proof toggle not available for this query type: ${error.message}`);
      return false;
    }
  }

  /**
   * Disable proof information toggle
   */
  async disableProofInfo() {
    // Wait a moment for the UI to fully load after query setup
    await this.page.waitForTimeout(1000);
    
    const proofContainer = this.page.locator(this.selectors.proofToggleContainer);
    
    // Check if proof container exists and becomes visible
    try {
      // Wait longer and check if container becomes visible or is already attached
      await proofContainer.waitFor({ state: 'attached', timeout: 10000 });
      
      // Check if it's visible or can be made visible
      const isVisible = await proofContainer.isVisible();
      if (!isVisible) {
        // It might be hidden by display:none, check if it exists in the DOM
        const count = await proofContainer.count();
        if (count === 0) {
          console.log('⚠️ Proof toggle container not found in DOM');
          return false;
        }
        
        // Try to wait a bit more for it to become visible
        try {
          await proofContainer.waitFor({ state: 'visible', timeout: 3000 });
        } catch {
          console.log('⚠️ Proof toggle container exists but remains hidden - may not be available for this query type');
          return false;
        }
      }
      
      const proofToggle = this.page.locator(this.selectors.proofToggle);
      await proofToggle.waitFor({ state: 'visible', timeout: 3000 });
      
      // Check if already disabled
      const isChecked = await proofToggle.isChecked();
      if (isChecked) {
        await proofToggle.uncheck();
      }
      
      console.log('✅ Proof info disabled');
      return true;
    } catch (error) {
      console.log(`⚠️ Proof toggle not available for this query type: ${error.message}`);
      return false;
    }
  }

  /**
   * Get the query description text
   */
  async getQueryDescription() {
    const description = this.page.locator('#queryDescription');
    if (await description.count() > 0) {
      return await description.textContent();
    }
    return null;
  }

  /**
   * Check if authentication inputs are visible
   */
  async hasAuthenticationInputs() {
    const authInputs = this.page.locator(this.selectors.authenticationInputs);
    return await authInputs.isVisible();
  }

  /**
   * Fill authentication information
   */
  async fillAuthentication(identityId, privateKey, assetLockProof = null) {
    if (await this.hasAuthenticationInputs()) {
      if (identityId) {
        await this.fillInput(this.selectors.identityId, identityId);
      }
      if (privateKey) {
        await this.fillInput(this.selectors.privateKey, privateKey);
      }
      if (assetLockProof) {
        await this.fillInput(this.selectors.assetLockProof, assetLockProof);
      }
      console.log('✅ Authentication filled');
    }
  }

  /**
   * Get current status banner state
   */
  async getStatusBannerState() {
    const banner = this.page.locator(this.selectors.statusBanner);
    const classList = await banner.getAttribute('class');
    
    if (classList.includes('success')) return 'success';
    if (classList.includes('error')) return 'error';
    if (classList.includes('loading')) return 'loading';
    return 'unknown';
  }

  /**
   * Get status banner text
   */
  async getStatusBannerText() {
    const banner = this.page.locator(this.selectors.statusBanner);
    return await banner.textContent();
  }

  /**
   * Wait for query execution to complete and return the result
   */
  async executeQueryAndGetResult(expectSuccess = true) {
    const success = await this.executeQuery();
    const result = await this.getResultContent();
    const hasError = await this.hasErrorResult();
    
    return {
      success,
      result,
      hasError,
      statusText: await this.getStatusBannerText()
    };
  }

  /**
   * Configure advanced SDK settings
   */
  async configureAdvancedSDK(options) {
    // Open SDK config if it's closed
    const configDetails = this.page.locator(this.selectors.sdkConfigDetails);
    const isOpen = await configDetails.getAttribute('open') !== null;
    
    if (!isOpen) {
      await configDetails.locator('summary').click();
    }
    
    // Fill configuration options
    if (options.platformVersion) {
      await this.fillInput(this.selectors.platformVersion, options.platformVersion);
    }
    if (options.connectTimeout) {
      await this.fillInput(this.selectors.connectTimeout, options.connectTimeout);
    }
    if (options.requestTimeout) {
      await this.fillInput(this.selectors.requestTimeout, options.requestTimeout);
    }
    if (options.retries) {
      await this.fillInput(this.selectors.retries, options.retries);
    }
    if (options.banFailedAddress !== undefined) {
      const checkbox = this.page.locator(this.selectors.banFailedAddress);
      if (options.banFailedAddress) {
        await checkbox.check();
      } else {
        await checkbox.uncheck();
      }
    }
    
    // Apply configuration
    await this.clickButton(this.selectors.applyConfig);
    
    console.log('✅ Advanced SDK configuration applied');
  }

  /**
   * Get available query categories
   */
  async getAvailableQueryCategories() {
    const categorySelect = this.page.locator(this.selectors.queryCategory);
    const options = await categorySelect.locator('option').allTextContents();
    return options.filter(option => option.trim() !== '' && option !== 'Select Query Category');
  }

  /**
   * Get available query types for current category
   */
  async getAvailableQueryTypes() {
    const queryTypeSelect = this.page.locator(this.selectors.queryType);
    await queryTypeSelect.waitFor({ state: 'visible' });
    const options = await queryTypeSelect.locator('option').allTextContents();
    return options.filter(option => option.trim() !== '' && option !== 'Select Query Type');
  }
}

module.exports = { WasmSdkPage };
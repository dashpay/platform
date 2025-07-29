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
      // Handle array inputs (like token IDs)
      await inputElement.fill(JSON.stringify(value));
    } else if (typeof value === 'object') {
      // Handle object inputs (JSON)
      await inputElement.fill(JSON.stringify(value));
    } else {
      // Handle text/number inputs
      await inputElement.fill(value.toString());
    }
  }

  /**
   * Enable proof information toggle
   */
  async enableProofInfo() {
    const proofToggle = this.page.locator(this.selectors.proofToggle);
    if (await proofToggle.count() > 0) {
      await proofToggle.check();
      console.log('✅ Proof info enabled');
    }
  }

  /**
   * Disable proof information toggle
   */
  async disableProofInfo() {
    const proofToggle = this.page.locator(this.selectors.proofToggle);
    if (await proofToggle.count() > 0) {
      await proofToggle.uncheck();
      console.log('✅ Proof info disabled');
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
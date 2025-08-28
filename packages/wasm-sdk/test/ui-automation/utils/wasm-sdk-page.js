const { BaseTest } = require('./base-test');
const { expect } = require('@playwright/test');

// Configuration for dynamic array parameters that require special handling
const DYNAMIC_ARRAY_PARAMETERS = {
  'ids': true,
  'identityIds': true,
  'identitiesIds': true,
  'tokenIds': true,
  'indexValues': true
};

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
    // Special handling for multiselect checkboxes (like purposes)
    if (paramName === 'purposes' && Array.isArray(value)) {
      for (const purposeValue of value) {
        const checkboxSelector = `input[name="purposes_${purposeValue}"][type="checkbox"]`;
        const checkbox = this.page.locator(checkboxSelector);
        if (await checkbox.count() > 0) {
          await checkbox.check();
        }
      }
      return;
    }
    
    // Special handling for array parameters that use dynamic input fields
    if (DYNAMIC_ARRAY_PARAMETERS[paramName]) {
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
        `label:has-text("${paramName}") + textarea`,
        // Special cases for contract and document fields
        `input[placeholder*="Contract ID"]`,
        `input[placeholder*="Document Type"]`,
        `textarea[placeholder*="JSON"]`,
        `textarea[placeholder*="Schema"]`
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
        console.warn(`⚠️  Could not find input for parameter: ${paramName}. Trying by label text...`);
        
        // Try finding by label text as last resort
        const labelSelectors = [
          `label:text-is("${paramName}") + input`,
          `label:text-is("${paramName}") + textarea`,
          `label:text-is("${paramName}") + select`
        ];
        
        for (const selector of labelSelectors) {
          const labelInput = this.page.locator(selector).first();
          if (await labelInput.count() > 0) {
            await this.fillInputByType(labelInput, value);
            found = true;
            break;
          }
        }
        
        if (!found) {
          console.warn(`⚠️  Could not find input for parameter: ${paramName} - skipping`);
        }
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
   * Helper method to toggle proof information
   * @param {boolean} enable - true to enable, false to disable
   * @returns {boolean} - true if successful, false if proof toggle not available
   */
  async _toggleProofInfo(enable) {
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
      
      // Check current state and toggle if needed
      const isChecked = await proofToggle.isChecked();
      const needsToggle = enable ? !isChecked : isChecked;
      
      if (needsToggle) {
        // Click on the toggle switch container or label to toggle it
        // Since it's a custom toggle, we need to click the label or toggle-slider
        const toggleLabel = proofContainer.locator('label');
        await toggleLabel.click();
        
        // Wait for the toggle to reach the expected state
        if (enable) {
          await expect(proofToggle).toBeChecked();
          console.log('Proof toggle confirmed as checked');
        } else {
          await expect(proofToggle).not.toBeChecked();
          console.log('Proof toggle confirmed as unchecked');
        }
      }
      
      console.log(`Proof info ${enable ? 'enabled' : 'disabled'}`);
      return true;
    } catch (error) {
      console.log(`⚠️ Proof toggle not available for this query type: ${error.message}`);
      return false;
    }
  }

  /**
   * Enable proof information toggle
   */
  async enableProofInfo() {
    return this._toggleProofInfo(true);
  }

  /**
   * Disable proof information toggle
   */
  async disableProofInfo() {
    return this._toggleProofInfo(false);
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
      console.log('Authentication filled');
    }
  }

  /**
   * Get current status banner state
   */
  async getStatusBannerState() {
    const banner = this.page.locator(this.selectors.statusBanner);
    const classList = await banner.getAttribute('class');
    
    // Handle null classList gracefully
    if (!classList) return 'unknown';
    
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
   * Get proof content when in split view mode
   */
  async getProofContent() {
    await this.page.waitForTimeout(500); // Brief wait for content to render
    
    const proofContent = this.page.locator('#proofInfo');
    const isVisible = await proofContent.isVisible();
    
    if (!isVisible) {
      console.log('⚠️ Proof content not visible');
      return '';
    }
    
    const content = await proofContent.textContent();
    return content || '';
  }

  /**
   * Check if result is displayed in split view (proof mode)
   */
  async isInSplitView() {
    const dataSection = this.page.locator('.result-data-section');
    const proofSection = this.page.locator('.result-proof-section');
    
    const dataSectionVisible = await dataSection.isVisible();
    const proofSectionVisible = await proofSection.isVisible();
    
    return dataSectionVisible && proofSectionVisible;
  }

  /**
   * Wait for query execution to complete and return the result
   */
  async executeQueryAndGetResult() {
    const success = await this.executeQuery();
    const result = await this.getResultContent();
    const hasError = await this.hasErrorResult();
    
    // Check if we're in split view mode (proof mode)
    const inSplitView = await this.isInSplitView();
    let proofContent = null;
    
    if (inSplitView) {
      proofContent = await this.getProofContent();
    }
    
    return {
      success,
      result,
      hasError,
      statusText: await this.getStatusBannerText(),
      inSplitView,
      proofContent
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
    
    console.log('Advanced SDK configuration applied');
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

  /**
   * Set up a state transition test scenario
   */
  async setupStateTransition(category, transitionType, parameters = {}) {
    // Set operation type to transitions
    await this.setOperationType('transitions');
    
    // Set category and transition type
    await this.setQueryCategory(category);
    await this.setQueryType(transitionType);
    
    // Fill in parameters
    if (Object.keys(parameters).length > 0) {
      await this.fillStateTransitionParameters(parameters);
    }
    
    return this;
  }

  /**
   * Fill state transition parameters
   */
  async fillStateTransitionParameters(parameters) {
    // Handle state transition specific parameters
    for (const [key, value] of Object.entries(parameters)) {
      if (key === 'assetLockProof') {
        await this.fillAssetLockProof(value);
      } else if (key === 'privateKey') {
        await this.fillPrivateKey(value);
      } else if (key === 'identityId') {
        await this.fillIdentityId(value);
      } else if (key === 'seedPhrase') {
        await this.fillSeedPhrase(value);
      } else if (key === 'identityIndex') {
        await this.fillIdentityIndex(value);
      } else if (key === 'keySelectionMode') {
        // Skip keySelectionMode for now - only needed for identity create
        console.log('Skipping keySelectionMode field (identity create only)');
      } else if (key === 'documentFields') {
        // Handle document fields - these need to be filled after schema fetch
        console.log('Document fields will be handled after schema fetch');
      } else if (key === 'description') {
        // Skip description field - it's just for documentation
        console.log('Skipping description field (documentation only)');
      } else {
        // Use the general parameter filling method for other parameters
        await this.fillParameterByName(key, value);
      }
    }
  }

  /**
   * Fill asset lock proof field
   */
  async fillAssetLockProof(assetLockProof) {
    await this.fillInput(this.selectors.assetLockProof, assetLockProof);
    console.log('Asset lock proof filled');
  }

  /**
   * Fill private key field
   */
  async fillPrivateKey(privateKey) {
    await this.fillInput(this.selectors.privateKey, privateKey);
    console.log('Private key filled');
  }

  /**
   * Fill identity ID field (for top-up transitions)
   */
  async fillIdentityId(identityId) {
    await this.fillInput(this.selectors.identityId, identityId);
    console.log('Identity ID filled');
  }

  /**
   * Fill seed phrase field
   */
  async fillSeedPhrase(seedPhrase) {
    const seedPhraseInput = this.page.locator('textarea[name="seedPhrase"]');
    await seedPhraseInput.fill(seedPhrase);
    console.log('Seed phrase filled');
  }

  /**
   * Fill identity index field
   */
  async fillIdentityIndex(identityIndex) {
    const identityIndexInput = this.page.locator('input[name="identityIndex"]');
    await identityIndexInput.fill(identityIndex.toString());
    console.log('Identity index filled');
  }

  /**
   * Set key selection mode (simple/advanced)
   */
  async setKeySelectionMode(mode) {
    const keySelectionSelect = this.page.locator('select[name="keySelectionMode"]');
    await keySelectionSelect.selectOption(mode);
    console.log(`Key selection mode set to: ${mode}`);
  }

  /**
   * Execute state transition and get result (similar to executeQueryAndGetResult)
   */
  async executeStateTransitionAndGetResult() {
    const success = await this.executeQuery(); // Same execute button works for transitions
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
   * Check if state transition authentication inputs are visible
   */
  async hasStateTransitionAuthInputs() {
    const authInputs = this.page.locator(this.selectors.authenticationInputs);
    const assetLockProofGroup = this.page.locator('#assetLockProofGroup');
    
    const authVisible = await authInputs.isVisible();
    const assetLockVisible = await assetLockProofGroup.isVisible();
    
    return authVisible && assetLockVisible;
  }

  /**
   * Fetch document schema and generate dynamic fields for document transitions
   */
  async fetchDocumentSchema() {
    console.log('Attempting to fetch document schema...');
    
    // First check if the function exists and call it directly
    try {
      await this.page.evaluate(() => {
        if (typeof window.fetchDocumentSchema === 'function') {
          return window.fetchDocumentSchema();
        } else {
          throw new Error('fetchDocumentSchema function not found');
        }
      });
      console.log('Called fetchDocumentSchema function directly');
    } catch (error) {
      console.error('Error calling fetchDocumentSchema:', error);
      throw error;
    }
    
    // Wait for schema to load and fields to be generated
    await this.page.waitForTimeout(3000);
    
    // Check if dynamic fields container is visible
    const dynamicFieldsContainer = this.page.locator('#dynamic_documentFields');
    await dynamicFieldsContainer.waitFor({ state: 'visible', timeout: 15000 });
    
    console.log('Document schema fetched and fields generated');
  }

  /**
   * Fill a specific document field by name
   */
  async fillDocumentField(fieldName, value) {
    const fieldInput = this.page.locator(`#dynamic_documentFields input[data-field-name="${fieldName}"], #dynamic_documentFields textarea[data-field-name="${fieldName}"]`);
    
    // Convert value to string based on type
    let stringValue = '';
    if (value === null || value === undefined) {
      stringValue = '';
    } else if (typeof value === 'object') {
      stringValue = JSON.stringify(value);
    } else {
      stringValue = value.toString();
    }
    
    await fieldInput.fill(stringValue);
    console.log(`Document field '${fieldName}' filled with value: ${stringValue}`);
  }

  /**
   * Fill multiple document fields
   */
  async fillDocumentFields(fields) {
    for (const [fieldName, value] of Object.entries(fields)) {
      await this.fillDocumentField(fieldName, value);
    }
    console.log('All document fields filled');
  }

  /**
   * Load existing document for replacement (gets revision and populates fields)
   */
  async loadExistingDocument() {
    console.log('Loading existing document for replacement...');
    
    // Call the loadExistingDocument function directly via page.evaluate
    try {
      await this.page.evaluate(() => {
        if (typeof window.loadExistingDocument === 'function') {
          return window.loadExistingDocument();
        } else {
          throw new Error('loadExistingDocument function not found');
        }
      });
      console.log('Existing document loaded successfully');
    } catch (error) {
      console.error('Error loading existing document:', error);
      throw error;
    }
    
    // Wait for the document to be loaded and fields to be populated
    await this.page.waitForTimeout(3000);
    
    console.log('Document loaded and fields populated');
  }

  /**
   * Fill complete state transition authentication (asset lock proof + private key)
   */
  async fillStateTransitionAuthentication(assetLockProof, privateKey, identityId = null) {
    if (await this.hasStateTransitionAuthInputs()) {
      if (assetLockProof) {
        await this.fillAssetLockProof(assetLockProof);
      }
      if (privateKey) {
        await this.fillPrivateKey(privateKey);
      }
      if (identityId) {
        await this.fillIdentityId(identityId);
      }
      console.log('State transition authentication filled');
    }
  }
}

module.exports = { WasmSdkPage };

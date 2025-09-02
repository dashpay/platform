const { testData, getTestParameters, getAllTestParameters, getStateTransitionParameters, getAllStateTransitionParameters } = require('../fixtures/test-data');

/**
 * Parameter injection system for WASM SDK UI tests
 * Maps test data to UI form fields automatically
 */
class ParameterInjector {
  constructor(wasmSdkPage) {
    this.page = wasmSdkPage;
    this.testData = testData;
  }

  /**
   * Inject parameters for a specific query based on test data
   */
  async injectParameters(category, queryType, network = 'testnet', parameterSetIndex = 0) {
    try {
      const allParameters = getAllTestParameters(category, queryType, network);
      
      if (allParameters.length === 0) {
        console.warn(`⚠️  No test parameters found for ${category}.${queryType} on ${network}`);
        return false;
      }

      const parameters = allParameters[parameterSetIndex] || allParameters[0];
      console.log(`📝 Injecting parameters for ${category}.${queryType}`);

      await this.page.fillQueryParameters(parameters);
      return true;
    } catch (error) {
      console.error(`❌ Failed to inject parameters for ${category}.${queryType}:`, error.message);
      return false;
    }
  }

  /**
   * Inject parameters for a specific state transition based on test data
   */
  async injectStateTransitionParameters(category, transitionType, network = 'testnet', customParams = {}) {
    try {
      // Get base parameters from test data
      const allParameters = getAllStateTransitionParameters(category, transitionType, network);
      
      if (allParameters.length === 0) {
        console.warn(`⚠️  No state transition test parameters found for ${category}.${transitionType} on ${network}`);
        return false;
      }
      
      const baseParameters = allParameters[0];
      
      // Merge base parameters with custom overrides
      const parameters = { ...baseParameters, ...customParams };
      
      console.log(`📝 Injecting state transition parameters for ${category}.${transitionType}`);

      await this.page.fillStateTransitionParameters(parameters);
      return true;
    } catch (error) {
      console.error(`❌ Failed to inject state transition parameters for ${category}.${transitionType}:`, error.message);
      return false;
    }
  }

  /**
   * Get parameter mapping for manual field filling
   * Maps parameter names to likely field selectors
   */
  getParameterFieldMapping() {
    return {
      // Identity parameters
      'id': ['#id', '[name="id"]', 'input[placeholder*="Identity ID"]'],
      'identityId': ['[name="identityId"]', '#identityId', 'input[placeholder*="Identity ID"]'],
      'identityIds': ['input[placeholder="Enter value"]', '.array-input-container input[type="text"]', '[data-array-name="identityIds"] input[type="text"]', '.array-input-container[data-array-name="identityIds"] input', '#identityIds', '[name="identityIds"]', 'input[placeholder*="Identity IDs"]'],
      'identitiesIds': ['input[placeholder="Enter value"]', '.array-input-container input[type="text"]', '[data-array-name="identitiesIds"] input[type="text"]', '.array-input-container[data-array-name="identitiesIds"] input', '#identitiesIds', '[name="identitiesIds"]', 'input[placeholder*="Identity IDs"]'],
      
      // Data contract parameters
      'dataContractId': ['#dataContractId', '[name="dataContractId"]', 'input[placeholder*="Contract ID"]'],
      'contractId': ['#contractId', '[name="contractId"]', 'input[placeholder*="Contract ID"]'],
      'ids': ['input[placeholder="Enter value"]', '.array-input-container input[type="text"]', '[data-array-name="ids"] input[type="text"]', '.array-input-container[data-array-name="ids"] input', '#ids', '[name="ids"]', 'input[placeholder*="Contract IDs"]', 'input[placeholder*="Data Contract ID"]'],
      
      // Document parameters
      'documentType': ['#documentType', '[name="documentType"]', 'input[placeholder*="Document Type"]'],
      'documentId': ['#documentId', '[name="documentId"]', 'input[placeholder*="Document ID"]'],
      
      // Key parameters
      'publicKeyHash': ['#publicKeyHash', '[name="publicKeyHash"]', 'input[placeholder*="Public Key Hash"]'],
      'keyRequestType': ['#keyRequestType', '[name="keyRequestType"]', 'select[name="keyRequestType"]'],
      'specificKeyIds': ['#specificKeyIds', '[name="specificKeyIds"]'],
      
      // Token parameters
      'tokenId': ['#tokenId', '[name="tokenId"]', 'input[placeholder*="Token ID"]'],
      'tokenIds': ['input[placeholder="Enter value"]', '.array-input-container input[type="text"]', '[data-array-name="tokenIds"] input[type="text"]', '.array-input-container[data-array-name="tokenIds"] input', '#tokenIds', '[name="tokenIds"]', 'input[placeholder*="Token IDs"]'],
      
      // DPNS parameters
      'label': ['#label', '[name="label"]', 'input[placeholder*="Username"]', 'input[placeholder*="Label"]'],
      'name': ['#name', '[name="name"]', 'input[placeholder*="Name"]', 'input[placeholder*="DPNS"]'],
      'prefix': ['#prefix', '[name="prefix"]', 'input[placeholder*="prefix"]', 'input[placeholder*="Prefix"]'],
      
      // Query modifiers
      'limit': ['#limit', '[name="limit"]', 'input[placeholder*="limit" i]'],
      'offset': ['#offset', '[name="offset"]', 'input[placeholder*="offset" i]'],
      'count': ['#count', '[name="count"]', 'input[placeholder*="count" i]'],
      
      // Epoch parameters
      'epoch': ['#epoch', '[name="epoch"]', 'input[placeholder*="epoch" i]'],
      'startEpoch': ['#startEpoch', '[name="startEpoch"]'],
      'ascending': ['#ascending', '[name="ascending"]', 'input[type="checkbox"][name="ascending"]'],
      'orderAscending': ['#orderAscending', '[name="orderAscending"]', 'input[type="checkbox"][name="orderAscending"]'],
      'startAfter': ['#startAfter', '[name="startAfter"]', 'input[placeholder*="startAfter" i]'],
      
      // ProTx parameters
      'startProTxHash': ['#startProTxHash', '[name="startProTxHash"]'],
      'proTxHashes': ['#proTxHashes', '[name="proTxHashes"]'],
      
      // Where clause and ordering
      'whereClause': ['#whereClause', '[name="whereClause"]', 'textarea[placeholder*="Where"]'],
      'orderBy': ['#orderBy', '[name="orderBy"]', 'textarea[placeholder*="Order"]'],
      
      // Voting parameters
      'documentTypeName': ['#documentTypeName', '[name="documentTypeName"]'],
      'indexName': ['#indexName', '[name="indexName"]'],
      'indexValues': ['#indexValues', '[name="indexValues"]', 'textarea[name="indexValues"]', 'input[placeholder*="indexValues"]'],
      'resultType': ['#resultType', '[name="resultType"]'],
      'contestantId': ['#contestantId', '[name="contestantId"]'],
      'allowIncludeLockedAndAbstainingVoteTally': ['#allowIncludeLockedAndAbstainingVoteTally', '[name="allowIncludeLockedAndAbstainingVoteTally"]', 'input[type="checkbox"][name="allowIncludeLockedAndAbstainingVoteTally"]'],
      'startAtIdentifierInfo': ['#startAtIdentifierInfo', '[name="startAtIdentifierInfo"]'],
      
      // Group parameters
      'contractId': ['#contractId', '[name="contractId"]', 'input[placeholder*="Contract ID"]'],
      'groupContractPosition': ['#groupContractPosition', '[name="groupContractPosition"]'],
      'startAtGroupContractPosition': ['#startAtGroupContractPosition', '[name="startAtGroupContractPosition"]'],
      'startGroupContractPositionIncluded': ['#startGroupContractPositionIncluded', '[name="startGroupContractPositionIncluded"]', 'input[type="checkbox"][name="startGroupContractPositionIncluded"]'],
      'status': ['#status', '[name="status"]', 'select[name="status"]'],
      'actionId': ['#actionId', '[name="actionId"]'],
      'startActionId': ['#startActionId', '[name="startActionId"]'],
      'startActionIdIncluded': ['#startActionIdIncluded', '[name="startActionIdIncluded"]', 'input[type="checkbox"][name="startActionIdIncluded"]'],
      
      // Time parameters
      'startTimeMs': ['#startTimeMs', '[name="startTimeMs"]'],
      'endTimeMs': ['#endTimeMs', '[name="endTimeMs"]']
    };
  }

  /**
   * Auto-detect and fill parameters using intelligent field matching
   */
  async autoFillParameters(parameters) {
    const fieldMapping = this.getParameterFieldMapping();
    const filledFields = [];
    const failedFields = [];

    for (const [paramName, value] of Object.entries(parameters)) {
      const success = await this.tryFillParameter(paramName, value, fieldMapping);
      
      if (success) {
        filledFields.push(paramName);
      } else {
        failedFields.push(paramName);
      }
    }

    console.log(`Successfully filled fields: ${filledFields.join(', ')}`);
    if (failedFields.length > 0) {
      console.warn(`⚠️  Failed to fill fields: ${failedFields.join(', ')}`);
    }

    return { filledFields, failedFields };
  }

  /**
   * Try to fill a parameter using multiple selector strategies
   */
  async tryFillParameter(paramName, value, fieldMapping) {
    const possibleSelectors = fieldMapping[paramName] || [];
    
    // Add generic fallback selectors
    possibleSelectors.push(
      `#${paramName}`,
      `[name="${paramName}"]`,
      `input[placeholder*="${paramName}" i]`,
      `label:has-text("${paramName}") + input`,
      `label:has-text("${paramName}") + select`,
      `label:has-text("${paramName}") + textarea`
    );

    for (const selector of possibleSelectors) {
      try {
        const element = this.page.page.locator(selector).first();
        const count = await element.count();
        
        if (count > 0) {
          const isVisible = await element.isVisible();
          
          if (isVisible) {
            await this.page.fillInputByType(element, value);
            console.log(`📝 Filled ${paramName} using selector: ${selector}`);
            return true;
          }
        }
      } catch (error) {
        continue;
      }
    }

    return false;
  }

  /**
   * Create test parameter sets for parameterized testing
   */
  createParameterizedTests(category, queryType, network = 'testnet') {
    const allParameters = getAllTestParameters(category, queryType, network);
    
    return allParameters.map((params, index) => ({
      testName: `${category}.${queryType} - Test Set ${index + 1}`,
      parameters: params,
      category,
      queryType,
      network,
      index
    }));
  }

  /**
   * Validate parameters against expected schema
   */
  validateParameters(parameters, expectedSchema = {}) {
    const validation = {
      valid: true,
      errors: [],
      warnings: []
    };

    for (const [key, value] of Object.entries(parameters)) {
      // Check for empty required values
      if (value === null || value === undefined || value === '') {
        validation.warnings.push(`Parameter '${key}' is empty`);
      }

      // Validate array parameters
      if (Array.isArray(value) && value.length === 0) {
        validation.warnings.push(`Array parameter '${key}' is empty`);
      }

      // Validate ID format (base58 compliance for Dash IDs)
      if (key.toLowerCase().includes('id') && typeof value === 'string') {
        if (!this.isValidBase58DashId(value)) {
          validation.errors.push(`Parameter '${key}' is not a valid base58-encoded Dash ID: ${value}`);
          validation.valid = false;
        }
      }
    }

    return validation;
  }

  /**
   * Validate if a string is a valid base58-encoded Dash ID
   * @param {string} id - The ID string to validate
   * @returns {boolean} - true if valid base58 Dash ID format
   */
  isValidBase58DashId(id) {
    // Dash IDs are typically 44 characters long when base58 encoded
    // Base58 alphabet: 123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
    // (excludes 0, O, I, l to avoid confusion)
    const base58Regex = /^[1-9A-HJ-NP-Za-km-z]{43,44}$/;
    
    if (!base58Regex.test(id)) {
      return false;
    }
    
    // Additional check: ensure it doesn't contain invalid base58 characters
    const invalidChars = /[0OIl]/;
    if (invalidChars.test(id)) {
      return false;
    }
    
    return true;
  }

  /**
   * Generate random test parameters for stress testing
   */
  generateRandomParameters(category, queryType) {
    // This would generate valid-looking but random parameters
    // for stress testing the UI with various inputs
    const generators = {
      id: () => 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', // Use known good ID
      identityId: () => 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
      dataContractId: () => 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
      limit: () => Math.floor(Math.random() * 100) + 1,
      count: () => Math.floor(Math.random() * 50) + 1,
      epoch: () => Math.floor(Math.random() * 10000) + 1000,
      ascending: () => Math.random() > 0.5
    };

    // Get base parameters and randomize some values
    try {
      const baseParams = getTestParameters(category, queryType, 'testnet');
      const randomized = { ...baseParams };

      for (const [key] of Object.entries(randomized)) {
        if (generators[key]) {
          randomized[key] = generators[key]();
        }
      }

      return randomized;
    } catch (error) {
      console.warn(`Could not generate random parameters for ${category}.${queryType}`);
      return {};
    }
  }
}

module.exports = { ParameterInjector };

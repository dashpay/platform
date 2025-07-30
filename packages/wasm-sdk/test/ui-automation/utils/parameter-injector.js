const { getTestParameters, getAllTestParameters } = require('../fixtures/test-data');

/**
 * Parameter injection system for WASM SDK UI tests
 * Maps test data to UI form fields automatically
 */
class ParameterInjector {
  constructor(wasmSdkPage) {
    this.page = wasmSdkPage;
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
      console.log(`📝 Injecting parameters for ${category}.${queryType}:`, parameters);

      await this.page.fillQueryParameters(parameters);
      return true;
    } catch (error) {
      console.error(`❌ Failed to inject parameters for ${category}.${queryType}:`, error.message);
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
      'identityId': ['#identityId', '[name="identityId"]', 'input[placeholder*="Identity ID"]'],
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
      
      // Query modifiers
      'limit': ['#limit', '[name="limit"]', 'input[placeholder*="limit" i]'],
      'offset': ['#offset', '[name="offset"]', 'input[placeholder*="offset" i]'],
      'count': ['#count', '[name="count"]', 'input[placeholder*="count" i]'],
      
      // Epoch parameters
      'epoch': ['#epoch', '[name="epoch"]', 'input[placeholder*="epoch" i]'],
      'startEpoch': ['#startEpoch', '[name="startEpoch"]'],
      'ascending': ['#ascending', '[name="ascending"]', 'input[type="checkbox"][name="ascending"]'],
      
      // ProTx parameters
      'startProTxHash': ['#startProTxHash', '[name="startProTxHash"]'],
      'proTxHashes': ['#proTxHashes', '[name="proTxHashes"]'],
      
      // Where clause and ordering
      'whereClause': ['#whereClause', '[name="whereClause"]', 'textarea[placeholder*="Where"]'],
      'orderBy': ['#orderBy', '[name="orderBy"]', 'textarea[placeholder*="Order"]'],
      
      // Voting parameters
      'documentTypeName': ['#documentTypeName', '[name="documentTypeName"]'],
      'indexName': ['#indexName', '[name="indexName"]'],
      'resultType': ['#resultType', '[name="resultType"]'],
      'contestantId': ['#contestantId', '[name="contestantId"]'],
      
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

      // Validate ID format (basic check)
      if (key.toLowerCase().includes('id') && typeof value === 'string') {
        if (value.length < 10) {
          validation.errors.push(`Parameter '${key}' appears to be too short for an ID: ${value}`);
          validation.valid = false;
        }
      }
    }

    return validation;
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

      for (const [key, value] of Object.entries(randomized)) {
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
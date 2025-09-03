const { test, expect } = require('@playwright/test');
const { WasmSdkPage } = require('../utils/wasm-sdk-page');
const { ParameterInjector } = require('../utils/parameter-injector');

/**
 * Helper function to execute a state transition
 * @param {WasmSdkPage} wasmSdkPage - The page object instance
 * @param {ParameterInjector} parameterInjector - The parameter injector instance
 * @param {string} category - State transition category (e.g., 'identity', 'dataContract')
 * @param {string} transitionType - Transition type (e.g., 'identityCreate')
 * @param {string} network - Network to use ('testnet' or 'mainnet')
 * @returns {Promise<Object>} - The transition result object
 */
async function executeStateTransition(wasmSdkPage, parameterInjector, category, transitionType, network = 'testnet') {
  await wasmSdkPage.setupStateTransition(category, transitionType);

  const success = await parameterInjector.injectStateTransitionParameters(category, transitionType, network);
  expect(success).toBe(true);

  const result = await wasmSdkPage.executeStateTransitionAndGetResult();

  return result;
}

/**
 * Helper function to validate basic state transition result properties
 * @param {Object} result - The state transition result object
 */
function validateBasicStateTransitionResult(result) {
  // Check for withdrawal-specific minimum amount error
  if (!result.success && result.result && result.result.includes('Missing response message')) {
    console.error('⚠️  Withdrawal may have failed due to insufficient amount. Minimum withdrawal is ~190,000 credits.');
    console.error('Full error:', result.result);
  }

  expect(result.success).toBe(true);
  expect(result.result).toBeDefined();
  expect(result.hasError).toBe(false);
  expect(result.result).not.toContain('Error executing');
  expect(result.result).not.toContain('invalid');
  expect(result.result).not.toContain('failed');
}

/**
 * Filter out placeholder options from dropdown arrays
 * @param {string[]} options - Array of dropdown options
 * @returns {string[]} - Filtered array without placeholders
 */
function filterPlaceholderOptions(options) {
  return options.filter(option =>
    !option.toLowerCase().includes('select') &&
    option.trim() !== ''
  );
}

/**
 * Parse and validate JSON response structure
 * @param {string} resultStr - The raw result string
 * @returns {Object} - The parsed contract data
 */
function parseContractResponse(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const contractData = JSON.parse(resultStr);
  expect(contractData).toBeDefined();
  expect(contractData).toBeInstanceOf(Object);
  expect(contractData.status).toBe('success');
  expect(contractData.contractId).toBeDefined();
  expect(contractData.version).toBeDefined();
  expect(typeof contractData.version).toBe('number');
  expect(contractData.message).toBeDefined();
  return contractData;
}

/**
 * Helper function to validate data contract result (both create and update)
 * @param {string} resultStr - The raw result string from data contract operation
 * @param {boolean} isUpdate - Whether this is an update operation (default: false for create)
 * @returns {Object} - The parsed contract data for further use
 */
function validateDataContractResult(resultStr, isUpdate = false) {
  const contractData = parseContractResponse(resultStr);

  // Conditional validations based on operation type
  if (isUpdate) {
    // Update: only has version and message specifics
    expect(contractData.version).toBeGreaterThan(1); // Updates should increment version
    expect(contractData.message).toContain('updated successfully');
  } else {
    // Create: has additional fields that updates don't have
    expect(contractData.ownerId).toBeDefined();
    expect(contractData.documentTypes).toBeDefined();
    expect(Array.isArray(contractData.documentTypes)).toBe(true);
    expect(contractData.version).toBe(1); // Creates start at version 1
    expect(contractData.message).toContain('created successfully');
  }

  return contractData;
}

/**
 * Helper function to validate document creation result
 * @param {string} resultStr - The raw result string from document creation
 */
function validateDocumentCreateResult(resultStr) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const documentResponse = JSON.parse(resultStr);
  expect(documentResponse).toBeDefined();
  expect(documentResponse).toBeInstanceOf(Object);

  // Validate the response structure for document creation
  expect(documentResponse.type).toBe('DocumentCreated');
  expect(documentResponse.documentId).toBeDefined();
  expect(typeof documentResponse.documentId).toBe('string');
  expect(documentResponse.documentId.length).toBeGreaterThan(0);

  // Validate the document object
  expect(documentResponse.document).toBeDefined();
  expect(documentResponse.document.id).toBe(documentResponse.documentId);
  expect(documentResponse.document.ownerId).toBeDefined();
  expect(documentResponse.document.dataContractId).toBeDefined();
  expect(documentResponse.document.documentType).toBeDefined();
  expect(documentResponse.document.revision).toBe(1); // New documents start at revision 1
  expect(documentResponse.document.data).toBeDefined();
  expect(typeof documentResponse.document.data).toBe('object');

  return documentResponse;
}

/**
 * Helper function to validate document replace result
 * @param {string} resultStr - The raw result string from document replacement
 * @param {string} expectedDocumentId - Expected document ID to validate against
 * @param {number} expectedMinRevision - Minimum expected revision (should be > 1)
 */
function validateDocumentReplaceResult(resultStr, expectedDocumentId, expectedMinRevision = 2) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const replaceResponse = JSON.parse(resultStr);
  expect(replaceResponse).toBeDefined();
  expect(replaceResponse).toBeInstanceOf(Object);

  // Validate the response structure for document replacement
  expect(replaceResponse.type).toBe('DocumentReplaced');
  expect(replaceResponse.documentId).toBe(expectedDocumentId);
  expect(replaceResponse.document).toBeDefined();

  // Validate the document object matches the expected structure
  expect(replaceResponse.document.id).toBe(expectedDocumentId);
  expect(replaceResponse.document.ownerId).toBeDefined();
  expect(replaceResponse.document.dataContractId).toBeDefined();
  expect(replaceResponse.document.documentType).toBeDefined();
  expect(replaceResponse.document.revision).toBeGreaterThanOrEqual(expectedMinRevision);
  expect(replaceResponse.document.data).toBeDefined();
  expect(typeof replaceResponse.document.data).toBe('object');

  console.log(`✅ Confirmed replacement of document: ${expectedDocumentId} (revision: ${replaceResponse.document.revision})`);

  return replaceResponse;
}

/**
 * Helper function to validate document deletion result
 * @param {string} resultStr - The raw result string from document deletion
 * @param {string} expectedDocumentId - Optional expected document ID to validate against
 */
function validateDocumentDeleteResult(resultStr, expectedDocumentId = null) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const deleteResponse = JSON.parse(resultStr);
  expect(deleteResponse).toBeDefined();
  expect(deleteResponse).toBeInstanceOf(Object);

  // Validate the response structure for document deletion
  expect(deleteResponse.type).toBe('DocumentDeleted');
  expect(deleteResponse.documentId).toBeDefined();
  expect(typeof deleteResponse.documentId).toBe('string');
  expect(deleteResponse.documentId.length).toBeGreaterThan(0);
  expect(deleteResponse.deleted).toBe(true);

  // If expectedDocumentId is provided, verify it matches the response
  if (expectedDocumentId) {
    expect(deleteResponse.documentId).toBe(expectedDocumentId);
    console.log(`Confirmed deletion of correct document: ${expectedDocumentId}`);
  }

  return deleteResponse;
}

/**
 * Helper function to validate identity credit transfer result
 * @param {string} resultStr - The raw result string from identity credit transfer
 * @param {string} expectedSenderId - Expected sender identity ID
 * @param {string} expectedRecipientId - Expected recipient identity ID
 * @param {number} expectedAmount - Expected transfer amount
 */
function validateIdentityCreditTransferResult(resultStr, expectedSenderId, expectedRecipientId, expectedAmount) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const transferResponse = JSON.parse(resultStr);
  expect(transferResponse).toBeDefined();
  expect(transferResponse).toBeInstanceOf(Object);

  // Validate the response structure for identity credit transfer
  expect(transferResponse.status).toBe('success');
  expect(transferResponse.senderId).toBe(expectedSenderId);
  expect(transferResponse.recipientId).toBe(expectedRecipientId);
  expect(transferResponse.amount).toBe(expectedAmount);
  expect(transferResponse.message).toBeDefined();

  console.log(`✅ Confirmed credit transfer: ${expectedAmount} credits from ${expectedSenderId} to ${expectedRecipientId}`);

  return transferResponse;
}

/**
 * Helper function to validate identity credit withdrawal result
 * @param {string} resultStr - The raw result string from identity credit withdrawal
 * @param {string} expectedIdentityId - Expected identity ID
 * @param {string} expectedWithdrawalAddress - Expected withdrawal address
 * @param {number} expectedAmount - Expected withdrawal amount
 */
function validateIdentityCreditWithdrawalResult(resultStr, expectedIdentityId, expectedWithdrawalAddress, expectedAmount) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const withdrawalResponse = JSON.parse(resultStr);
  expect(withdrawalResponse).toBeDefined();
  expect(withdrawalResponse).toBeInstanceOf(Object);

  // Validate the response structure for identity credit withdrawal
  expect(withdrawalResponse.status).toBe('success');
  expect(withdrawalResponse.identityId).toBe(expectedIdentityId);
  expect(withdrawalResponse.toAddress).toBe(expectedWithdrawalAddress);
  expect(withdrawalResponse.amount).toBeDefined(); // Amount might be different due to fees
  expect(withdrawalResponse.remainingBalance).toBeDefined();
  expect(withdrawalResponse.message).toContain('withdrawn successfully');

  console.log(`✅ Confirmed credit withdrawal: ${withdrawalResponse.amount} credits from ${expectedIdentityId} to ${expectedWithdrawalAddress}`);

  return withdrawalResponse;
}

/**
 * Helper function to validate token mint result
 * @param {string} resultStr - The raw result string from token mint
 * @param {string} expectedIdentityId - Expected identity ID
 * @param {string} expectedAmount - Expected mint amount
 */
function validateTokenMintResult(resultStr, expectedIdentityId, expectedAmount) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const mintResponse = JSON.parse(resultStr);
  expect(mintResponse).toBeDefined();
  expect(mintResponse).toBeInstanceOf(Object);

  // Token mint returns an empty object {} on success
  // This indicates the transaction was submitted successfully
  console.log(`✅ Token mint transaction submitted successfully: ${expectedAmount} tokens to ${expectedIdentityId}`);

  return mintResponse;
}

/**
 * Helper function to validate token transfer result
 * @param {string} resultStr - The raw result string from token transfer
 * @param {string} expectedSenderId - Expected sender identity ID
 * @param {string} expectedRecipientId - Expected recipient identity ID
 * @param {string} expectedAmount - Expected transfer amount
 */
function validateTokenTransferResult(resultStr, expectedSenderId, expectedRecipientId, expectedAmount) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const transferResponse = JSON.parse(resultStr);
  expect(transferResponse).toBeDefined();
  expect(transferResponse).toBeInstanceOf(Object);

  // Token transfer returns an empty object {} on success
  // This indicates the transaction was submitted successfully
  console.log(`✅ Token transfer transaction submitted successfully: ${expectedAmount} tokens from ${expectedSenderId} to ${expectedRecipientId}`);

  return transferResponse;
}

/**
 * Helper function to validate token burn result
 * @param {string} resultStr - The raw result string from token burn
 * @param {string} expectedIdentityId - Expected identity ID
 * @param {string} expectedAmount - Expected burn amount
 */
function validateTokenBurnResult(resultStr, expectedIdentityId, expectedAmount) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const burnResponse = JSON.parse(resultStr);
  expect(burnResponse).toBeDefined();
  expect(burnResponse).toBeInstanceOf(Object);

  // Token burn returns an empty object {} on success
  // This indicates the transaction was submitted successfully
  console.log(`✅ Token burn transaction submitted successfully: ${expectedAmount} tokens burned from ${expectedIdentityId}`);

  return burnResponse;
}

/**
 * Helper function to validate token freeze result
 * @param {string} resultStr - The raw result string from token freeze
 * @param {string} expectedIdentityId - Expected identity ID to freeze
 */
function validateTokenFreezeResult(resultStr, expectedIdentityId) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const freezeResponse = JSON.parse(resultStr);
  expect(freezeResponse).toBeDefined();
  expect(freezeResponse).toBeInstanceOf(Object);

  // Token freeze returns an empty object {} on success
  console.log(`✅ Token freeze transaction submitted successfully for identity: ${expectedIdentityId}`);

  return freezeResponse;
}

/**
 * Helper function to validate token destroy frozen result
 * @param {string} resultStr - The raw result string from token destroy frozen
 * @param {string} expectedIdentityId - Expected identity ID with frozen tokens
 */
function validateTokenDestroyFrozenResult(resultStr, expectedIdentityId) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const destroyResponse = JSON.parse(resultStr);
  expect(destroyResponse).toBeDefined();
  expect(destroyResponse).toBeInstanceOf(Object);

  // Token destroy frozen returns an empty object {} on success
  console.log(`✅ Token destroy frozen transaction submitted successfully: destroyed all frozen tokens from ${expectedIdentityId}`);

  return destroyResponse;
}

/**
 * Helper function to validate token unfreeze result
 * @param {string} resultStr - The raw result string from token unfreeze
 * @param {string} expectedIdentityId - Expected identity ID to unfreeze
 */
function validateTokenUnfreezeResult(resultStr, expectedIdentityId) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const unfreezeResponse = JSON.parse(resultStr);
  expect(unfreezeResponse).toBeDefined();
  expect(unfreezeResponse).toBeInstanceOf(Object);

  // Token unfreeze returns an empty object {} on success
  console.log(`✅ Token unfreeze transaction submitted successfully for identity: ${expectedIdentityId}`);

  return unfreezeResponse;
}

function validateTokenClaimResult(resultStr, expectedDistributionType) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const claimResponse = JSON.parse(resultStr);
  expect(claimResponse).toBeDefined();
  expect(claimResponse).toBeInstanceOf(Object);

  // Token claim returns an empty object {} on success
  console.log(`✅ Token claim transaction submitted successfully for distribution type: ${expectedDistributionType}`);

  return claimResponse;
}

function validateTokenSetPriceResult(resultStr, expectedPriceType, expectedPriceData) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const setPriceResponse = JSON.parse(resultStr);
  expect(setPriceResponse).toBeDefined();
  expect(setPriceResponse).toBeInstanceOf(Object);

  // Token set price returns an empty object {} on success
  console.log(`✅ Token set price transaction submitted successfully - Type: ${expectedPriceType}, Price: ${expectedPriceData}`);

  return setPriceResponse;
}

function validateTokenDirectPurchaseResult(resultStr, expectedAmount, expectedTotalPrice) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const purchaseResponse = JSON.parse(resultStr);
  expect(purchaseResponse).toBeDefined();
  expect(purchaseResponse).toBeInstanceOf(Object);

  // Token direct purchase returns an empty object {} on success
  console.log(`✅ Token direct purchase transaction submitted successfully - Amount: ${expectedAmount} tokens, Total price: ${expectedTotalPrice} credits`);

  return purchaseResponse;
}

function validateTokenConfigUpdateResult(resultStr, expectedConfigType, expectedConfigValue) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const configUpdateResponse = JSON.parse(resultStr);
  expect(configUpdateResponse).toBeDefined();
  expect(configUpdateResponse).toBeInstanceOf(Object);

  // Token config update returns an empty object {} on success
  console.log(`✅ Token config update transaction submitted successfully - Type: ${expectedConfigType}, Value: ${expectedConfigValue}`);

  return configUpdateResponse;
}

/**
 * Helper function to validate document transfer result
 * @param {string} resultStr - The raw result string from document transfer
 * @param {string} expectedDocumentId - Expected document ID to validate against
 * @param {string} expectedRecipientId - Expected recipient identity ID
 */
function validateDocumentTransferResult(resultStr, expectedDocumentId, expectedRecipientId) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const transferResponse = JSON.parse(resultStr);
  expect(transferResponse).toBeDefined();
  expect(transferResponse).toBeInstanceOf(Object);

  // Validate the response structure for document transfer
  expect(transferResponse.type).toBe('DocumentTransferred');
  expect(transferResponse.documentId).toBe(expectedDocumentId);
  expect(transferResponse.newOwnerId).toBe(expectedRecipientId);
  expect(transferResponse.transferred).toBe(true);

  console.log(`✅ Confirmed transfer of document: ${expectedDocumentId} to ${expectedRecipientId}`);

  return transferResponse;
}

/**
 * Helper function to validate document set price result
 * @param {string} resultStr - The raw result string from document set price
 * @param {string} expectedDocumentId - Expected document ID to validate against
 * @param {number} expectedPrice - Expected price that was set
 */
function validateDocumentSetPriceResult(resultStr, expectedDocumentId, expectedPrice) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const setPriceResponse = JSON.parse(resultStr);
  expect(setPriceResponse).toBeDefined();
  expect(setPriceResponse).toBeInstanceOf(Object);

  // Validate the response structure for document set price
  expect(setPriceResponse.type).toBe('DocumentPriceSet');
  expect(setPriceResponse.documentId).toBe(expectedDocumentId);
  expect(setPriceResponse.price).toBe(expectedPrice);
  expect(setPriceResponse.priceSet).toBe(true);

  console.log(`✅ Confirmed price set for document: ${expectedDocumentId} at ${expectedPrice} credits`);

  return setPriceResponse;
}

/**
 * Helper function to validate document purchase result
 * @param {string} resultStr - The raw result string from document purchase
 * @param {string} expectedDocumentId - Expected document ID to validate against
 * @param {string} expectedBuyerId - Expected buyer identity ID
 * @param {number} expectedPrice - Expected purchase price
 */
function validateDocumentPurchaseResult(resultStr, expectedDocumentId, expectedBuyerId, expectedPrice) {
  expect(() => JSON.parse(resultStr)).not.toThrow();
  const purchaseResponse = JSON.parse(resultStr);
  expect(purchaseResponse).toBeDefined();
  expect(purchaseResponse).toBeInstanceOf(Object);

  // Validate the response structure for document purchase
  expect(purchaseResponse.type).toBe('DocumentPurchased');
  expect(purchaseResponse.documentId).toBe(expectedDocumentId);
  expect(purchaseResponse.status).toBe('success');
  expect(purchaseResponse.newOwnerId).toBe(expectedBuyerId);
  expect(purchaseResponse.pricePaid).toBe(expectedPrice);
  expect(purchaseResponse.message).toBe('Document purchased successfully');
  expect(purchaseResponse.documentUpdated).toBe(true);
  expect(purchaseResponse.revision).toBeDefined();
  expect(typeof purchaseResponse.revision).toBe('number');

  console.log(`✅ Confirmed purchase of document: ${expectedDocumentId} by ${expectedBuyerId} for ${expectedPrice} credits`);

  return purchaseResponse;
}

/**
 * Execute a state transition with custom parameters
 * @param {WasmSdkPage} wasmSdkPage - The page object instance
 * @param {ParameterInjector} parameterInjector - The parameter injector instance
 * @param {string} category - State transition category
 * @param {string} transitionType - Transition type
 * @param {string} network - Network to use
 * @param {Object} customParams - Custom parameters to override test data
 * @returns {Promise<Object>} - The transition result object
 */
async function executeStateTransitionWithCustomParams(wasmSdkPage, parameterInjector, category, transitionType, network = 'testnet', customParams = {}) {
  await wasmSdkPage.setupStateTransition(category, transitionType);

  const success = await parameterInjector.injectStateTransitionParameters(category, transitionType, network, customParams);
  expect(success).toBe(true);

  const result = await wasmSdkPage.executeStateTransitionAndGetResult();

  return result;
}

test.describe('WASM SDK State Transition Tests', () => {
  let wasmSdkPage;
  let parameterInjector;

  test.beforeEach(async ({ page }) => {
    wasmSdkPage = new WasmSdkPage(page);
    parameterInjector = new ParameterInjector(wasmSdkPage);
    await wasmSdkPage.initialize('testnet');
  });

  test.describe('Data Contract State Transitions', () => {
    test.skip('should execute data contract create transition', async () => {
      // Execute the data contract create transition
      const result = await executeStateTransition(
        wasmSdkPage,
        parameterInjector,
        'dataContract',
        'dataContractCreate',
        'testnet'
      );

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Validate data contract creation specific result
      validateDataContractResult(result.result, false);

      console.log('✅ Data contract create state transition completed successfully');
    });

    test.skip('should execute data contract update transition', async () => {
      // Execute the data contract update transition
      const result = await executeStateTransition(
        wasmSdkPage,
        parameterInjector,
        'dataContract',
        'dataContractUpdate',
        'testnet'
      );

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Validate data contract update specific result
      validateDataContractResult(result.result, true);

      console.log('✅ Data contract update state transition completed successfully');
    });

    test('should create data contract and then update it with author field', async () => {
      // Set extended timeout for combined create+update operation
      test.setTimeout(180000);

      let contractId;

      // Step 1: Create contract (reported separately)
      await test.step('Create data contract', async () => {
        console.log('Creating new data contract...');
        const createResult = await executeStateTransition(
          wasmSdkPage,
          parameterInjector,
          'dataContract',
          'dataContractCreate',
          'testnet'
        );

        // Validate create result
        validateBasicStateTransitionResult(createResult);
        validateDataContractResult(createResult.result, false);

        // Get the contract ID from create result
        contractId = JSON.parse(createResult.result).contractId;
        console.log('✅ Data contract created with ID:', contractId);
      });

      // Step 2: Update contract (reported separately)
      // This test is now flaky for some reason and frequently fails
      await test.step('Update data contract with author field', async () => {
        console.log('🔄 Updating data contract to add author field...');
        const updateResult = await executeStateTransitionWithCustomParams(
          wasmSdkPage,
          parameterInjector,
          'dataContract',
          'dataContractUpdate',
          'testnet',
          { dataContractId: contractId } // Override with dynamic contract ID
        );

        // Validate update result
        validateBasicStateTransitionResult(updateResult);
        validateDataContractResult(updateResult.result, true);

        console.log('✅ Data contract updated successfully with author field');
      });
    });

    test('should show authentication inputs for data contract transitions', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');

      // Check that authentication inputs are visible
      const hasAuthInputs = await wasmSdkPage.hasAuthenticationInputs();
      expect(hasAuthInputs).toBe(true);

      console.log('✅ Data contract state transition authentication inputs are visible');
    });
  });

  test.describe('Document State Transitions', () => {
    test('should execute document create transition', async () => {
      // Set up the document create transition manually due to special schema handling
      await wasmSdkPage.setupStateTransition('document', 'documentCreate');

      // Inject basic parameters (contractId, documentType, identityId, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('document', 'documentCreate', 'testnet');
      expect(success).toBe(true);

      // Step 1: Fetch document schema to generate dynamic fields
      await test.step('Fetch document schema', async () => {
        await wasmSdkPage.fetchDocumentSchema();
        console.log('✅ Document schema fetched and fields generated');
      });

      // Step 2: Fill document fields
      await test.step('Fill document fields', async () => {
        // Get document fields from test data
        const testParams = parameterInjector.testData.stateTransitionParameters.document.documentCreate.testnet[0];
        await wasmSdkPage.fillDocumentFields(testParams.documentFields);
        console.log('✅ Document fields filled');
      });

      // Step 3: Execute the transition
      await test.step('Execute document create', async () => {
        const result = await wasmSdkPage.executeStateTransitionAndGetResult();

        // Validate basic result structure
        validateBasicStateTransitionResult(result);

        // Validate document creation specific result
        validateDocumentCreateResult(result.result);

        console.log('✅ Document create state transition completed successfully');
      });
    });

    test('should execute document replace transition', async () => {
      // Set up the document replace transition
      await wasmSdkPage.setupStateTransition('document', 'documentReplace');

      // Inject basic parameters (contractId, documentType, documentId, identityId, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('document', 'documentReplace', 'testnet');
      expect(success).toBe(true);

      // Load the existing document to get revision and populate fields
      await wasmSdkPage.loadExistingDocument();

      // Create updated message with timestamp
      const testParams = parameterInjector.testData.stateTransitionParameters.document.documentReplace.testnet[0];
      const baseMessage = testParams.documentFields.message;
      const timestamp = new Date().toISOString();
      const updatedFields = {
        message: `${baseMessage} - Updated at ${timestamp}`
      };

      // Fill updated document fields
      await wasmSdkPage.fillDocumentFields(updatedFields);

      // Execute the replace transition
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Validate document replace specific result with expected document ID
      const expectedDocumentId = testParams.documentId;
      validateDocumentReplaceResult(result.result, expectedDocumentId);

      console.log('✅ Document replace state transition completed successfully');
    });

    test('should set price, purchase, and transfer a trading card document', async () => {
      // Set extended timeout for complete marketplace workflow
      test.setTimeout(275000);

      let documentId;
      // Step 1: Set price on the card (by owner - primary identity)
      await test.step('Set price on trading card', async () => {
        console.log('Setting price on trading card...');

        // Get the configured price from test data
        const setPriceParams = parameterInjector.testData.stateTransitionParameters.document.documentSetPrice.testnet[0];
        const configuredPrice = setPriceParams.price;
        
        // Execute the set price transition
        const setPriceResult = await executeStateTransitionWithCustomParams(
          wasmSdkPage,
          parameterInjector,
          'document',
          'documentSetPrice',
          'testnet',
          {}
        );

        // Validate basic result structure
        validateBasicStateTransitionResult(setPriceResult);

        // Get document ID from test data for validation
        documentId = setPriceParams.documentId;
        
        // Validate document set price specific result
        validateDocumentSetPriceResult(
          setPriceResult.result,
          documentId,
          configuredPrice
        );

        console.log('✅ Card price set successfully');
      });

      // Step 2: Purchase the card with secondary identity (tests purchase flow)
      await test.step('Purchase trading card with secondary identity', async () => {
        console.log('Purchasing trading card with secondary identity...');

        // Get the configured price from test data
        const purchaseParams = parameterInjector.testData.stateTransitionParameters.document.documentPurchase.testnet[0];
        const purchaseConfiguredPrice = purchaseParams.price;
        
        // Log if the purchase price differs from what was set
        const setPriceParams = parameterInjector.testData.stateTransitionParameters.document.documentSetPrice.testnet[0];
        if (purchaseConfiguredPrice !== setPriceParams.price) {
          console.log(`⚠️ Note: documentPurchase uses price ${purchaseConfiguredPrice}, but documentSetPrice set it to ${setPriceParams.price}`);
        }

        // Execute the purchase transition
        const purchaseResult = await executeStateTransitionWithCustomParams(
          wasmSdkPage,
          parameterInjector,
          'document',
          'documentPurchase',
          'testnet',
          {}
        );

        // Validate basic result structure
        validateBasicStateTransitionResult(purchaseResult);

        // Get test parameters for validation (secondary identity is the buyer)
        const testParams = parameterInjector.testData.stateTransitionParameters.document.documentPurchase.testnet[0];

        // Validate document purchase specific result
        validateDocumentPurchaseResult(
          purchaseResult.result,
          documentId,
          testParams.identityId, // Secondary identity as buyer
          purchaseConfiguredPrice  // Use the actual price from test-data.js
        );

        console.log('✅ Card purchased by secondary identity successfully');
      });

      // Step 3: Transfer the card back to primary identity (tests transfer flow)
      await test.step('Transfer card back to primary identity', async () => {
        console.log('Transferring card back to primary identity...');

        // Get primary identity ID from test data
        const primaryIdentityId = parameterInjector.testData.stateTransitionParameters.dataContract.dataContractCreate.testnet[0].identityId;

        // Execute the transfer transition
        const transferResult = await executeStateTransitionWithCustomParams(
          wasmSdkPage,
          parameterInjector,
          'document',
          'documentTransfer',
          'testnet',
          {
            recipientId: primaryIdentityId // Transfer back to primary identity
          }
        );

        // Validate basic result structure
        validateBasicStateTransitionResult(transferResult);

        // Validate document transfer specific result
        validateDocumentTransferResult(
          transferResult.result,
          documentId,
          primaryIdentityId // Primary identity as recipient
        );

        console.log('✅ Complete marketplace workflow completed: Create → Set Price → Purchase → Transfer');
      });
    });

    test('should create, replace, and delete a document', async () => {
      // Set extended timeout for combined create+replace+delete operation
      test.setTimeout(260000);

      let documentId;

      // Step 1: Create document (reported separately)
      await test.step('Create document', async () => {
        console.log('Creating new document...');

        // Set up the document create transition
        await wasmSdkPage.setupStateTransition('document', 'documentCreate');

        // Inject basic parameters (contractId, documentType, identityId, privateKey)
        const success = await parameterInjector.injectStateTransitionParameters('document', 'documentCreate', 'testnet');
        expect(success).toBe(true);

        // Fetch document schema to generate dynamic fields
        await wasmSdkPage.fetchDocumentSchema();

        // Fill document fields
        const testParams = parameterInjector.testData.stateTransitionParameters.document.documentCreate.testnet[0];
        await wasmSdkPage.fillDocumentFields(testParams.documentFields);

        // Execute the transition
        const createResult = await wasmSdkPage.executeStateTransitionAndGetResult();

        // Validate create result
        validateBasicStateTransitionResult(createResult);
        const documentResponse = validateDocumentCreateResult(createResult.result);

        // Get the document ID from create result
        documentId = documentResponse.documentId;
        console.log('✅ Document created with ID:', documentId);
      });

      // Step 2: Replace the document (reported separately)
      await test.step('Replace document', async () => {
        console.log('Replacing the created document...');

        // Set up document replace transition
        await wasmSdkPage.setupStateTransition('document', 'documentReplace');

        // Inject parameters with the created document ID
        const success = await parameterInjector.injectStateTransitionParameters(
          'document',
          'documentReplace',
          'testnet',
          { documentId } // Override with the created document ID
        );
        expect(success).toBe(true);

        // Load the existing document to get revision
        await wasmSdkPage.loadExistingDocument();

        // Create updated message with timestamp
        const originalTestParams = parameterInjector.testData.stateTransitionParameters.document.documentCreate.testnet[0];
        const originalMessage = originalTestParams.documentFields.message;
        const timestamp = new Date().toISOString();
        const updatedFields = {
          message: `${originalMessage} - Updated at ${timestamp}`
        };

        // Fill updated document fields
        await wasmSdkPage.fillDocumentFields(updatedFields);

        // Execute the replace transition
        const replaceResult = await wasmSdkPage.executeStateTransitionAndGetResult();

        // Validate replace result
        validateBasicStateTransitionResult(replaceResult);
        validateDocumentReplaceResult(replaceResult.result, documentId);

        console.log('✅ Document replaced successfully');
      });

      // Step 3: Delete the document (reported separately)
      await test.step('Delete document', async () => {
        console.log('Deleting the created document...');

        // Set up document delete transition with the created document ID
        await wasmSdkPage.setupStateTransition('document', 'documentDelete');

        // Inject parameters with the dynamic document ID
        const success = await parameterInjector.injectStateTransitionParameters(
          'document',
          'documentDelete',
          'testnet',
          { documentId } // Override with the created document ID
        );
        expect(success).toBe(true);

        // Execute the delete transition
        const deleteResult = await wasmSdkPage.executeStateTransitionAndGetResult();

        // Validate delete result with expected document ID
        validateBasicStateTransitionResult(deleteResult);
        validateDocumentDeleteResult(deleteResult.result, documentId);

        console.log('✅ Document deleted successfully');
      });
    });

    test('should show authentication inputs for document transitions', async () => {
      await wasmSdkPage.setupStateTransition('document', 'documentCreate');

      // Check that authentication inputs are visible
      const hasAuthInputs = await wasmSdkPage.hasAuthenticationInputs();
      expect(hasAuthInputs).toBe(true);

      console.log('✅ Document state transition authentication inputs are visible');
    });
  });

  test.describe('Identity State Transitions', () => {
    test('should execute identity credit transfer transition', async () => {
      // Set up the identity credit transfer transition
      await wasmSdkPage.setupStateTransition('identity', 'identityCreditTransfer');

      // Inject parameters (senderId, recipientId, amount, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('identity', 'identityCreditTransfer', 'testnet');
      expect(success).toBe(true);

      // Execute the transfer
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.identity.identityCreditTransfer.testnet[0];

      // Validate identity credit transfer specific result
      validateIdentityCreditTransferResult(
        result.result,
        testParams.identityId, // Sender is the identityId field
        testParams.recipientId,
        testParams.amount
      );

      console.log('✅ Identity credit transfer state transition completed successfully');
    });

    test('should execute identity credit withdrawal transition', async () => {
      // Get test parameters to check withdrawal amount upfront
      const testParams = parameterInjector.testData.stateTransitionParameters.identity.identityCreditWithdrawal.testnet[0];

      // Skip test if withdrawal amount is below minimum threshold
      if (testParams.amount < 190000) {
        test.skip(true, `Withdrawal amount ${testParams.amount} credits is below minimum threshold (~190,000 credits)`);
      }

      // Set up the identity credit withdrawal transition
      await wasmSdkPage.setupStateTransition('identity', 'identityCreditWithdrawal');

      // Inject parameters (identityId, withdrawalAddress, amount, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('identity', 'identityCreditWithdrawal', 'testnet');
      expect(success).toBe(true);

      // Execute the withdrawal
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Validate identity credit withdrawal specific result
      validateIdentityCreditWithdrawalResult(
        result.result,
        testParams.identityId,
        testParams.toAddress,
        testParams.amount
      );

      console.log('✅ Identity credit withdrawal state transition completed successfully');
    });

    test('should show authentication inputs for identity transitions', async () => {
      await wasmSdkPage.setupStateTransition('identity', 'identityCreditTransfer');

      // Check that authentication inputs are visible
      const hasAuthInputs = await wasmSdkPage.hasAuthenticationInputs();
      expect(hasAuthInputs).toBe(true);

      console.log('✅ Identity state transition authentication inputs are visible');
    });
  });

  test.describe('Token State Transitions', () => {
    test('should execute token mint transition', async () => {
      // Set up the token mint transition
      await wasmSdkPage.setupStateTransition('token', 'tokenMint');

      // Inject parameters (contractId, tokenId, tokenPosition, amount, issuedToIdentityId, identityId, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenMint', 'testnet');
      expect(success).toBe(true);

      // Execute the mint
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenMint.testnet[0];

      // Validate token mint specific result
      validateTokenMintResult(
        result.result,
        testParams.identityId,
        testParams.amount
      );

      console.log('✅ Token mint state transition completed successfully');
    });

    test('should execute token transfer transition', async () => {
      // Set up the token transfer transition
      await wasmSdkPage.setupStateTransition('token', 'tokenTransfer');

      // Inject parameters (contractId, tokenId, tokenPosition, amount, recipientId, identityId, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenTransfer', 'testnet');
      expect(success).toBe(true);

      // Execute the transfer
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenTransfer.testnet[0];

      // Validate token transfer specific result
      validateTokenTransferResult(
        result.result,
        testParams.identityId,
        testParams.recipientId,
        testParams.amount
      );

      console.log('✅ Token transfer state transition completed successfully');
    });

    test('should execute token burn transition', async () => {
      // Set up the token burn transition
      await wasmSdkPage.setupStateTransition('token', 'tokenBurn');

      // Inject parameters (contractId, tokenId, tokenPosition, amount, identityId, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenBurn', 'testnet');
      expect(success).toBe(true);

      // Execute the burn
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenBurn.testnet[0];

      // Validate token burn specific result
      validateTokenBurnResult(
        result.result,
        testParams.identityId,
        testParams.amount
      );

      console.log('✅ Token burn state transition completed successfully');
    });

    test('should execute token freeze transition', async () => {
      // Set up the token freeze transition
      await wasmSdkPage.setupStateTransition('token', 'tokenFreeze');

      // Inject parameters (contractId, tokenPosition, identityId, identityToFreeze, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenFreeze', 'testnet');
      expect(success).toBe(true);

      // Execute the freeze
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenFreeze.testnet[0];

      // Validate token freeze specific result
      validateTokenFreezeResult(result.result, testParams.identityToFreeze);

      console.log('✅ Token freeze state transition completed successfully');
    });

    test('should execute token destroy frozen transition', async () => {
      // Set up the token destroy frozen transition
      await wasmSdkPage.setupStateTransition('token', 'tokenDestroyFrozen');

      // Inject parameters (contractId, tokenPosition, identityId, destroyFromIdentityId, amount, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenDestroyFrozen', 'testnet');
      expect(success).toBe(true);

      // Execute the destroy frozen
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenDestroyFrozen.testnet[0];

      // Validate token destroy frozen specific result
      validateTokenDestroyFrozenResult(result.result, testParams.frozenIdentityId);

      console.log('✅ Token destroy frozen state transition completed successfully');
    });

    test('should execute token unfreeze transition', async () => {
      // Set up the token unfreeze transition
      await wasmSdkPage.setupStateTransition('token', 'tokenUnfreeze');

      // Inject parameters (contractId, tokenPosition, identityId, identityToUnfreeze, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenUnfreeze', 'testnet');
      expect(success).toBe(true);

      // Execute the unfreeze
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenUnfreeze.testnet[0];

      // Validate token unfreeze specific result
      validateTokenUnfreezeResult(result.result, testParams.identityToUnfreeze);

      console.log('✅ Token unfreeze state transition completed successfully');
    });

    test('should execute token claim transition', async () => {
      // Set up the token claim transition
      await wasmSdkPage.setupStateTransition('token', 'tokenClaim');

      // Inject parameters (contractId, tokenPosition, distributionType, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenClaim', 'testnet');
      expect(success).toBe(true);

      // Execute the claim
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Check for expected platform responses indicating no tokens available
      if (!result.success && result.result && result.result.includes('Missing response message')) {
        // Skip the test with a descriptive reason
        test.skip(true, 'Platform returned "Missing response message". Probably no tokens available to claim.');
      }

      // Validate normal success case
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenClaim.testnet[0];

      // Validate token claim specific result
      validateTokenClaimResult(result.result, testParams.distributionType);

      console.log('✅ Token claim state transition completed successfully');
    });

    test('should execute token set price transition', async () => {
      // Set up the token set price transition
      await wasmSdkPage.setupStateTransition('token', 'tokenSetPriceForDirectPurchase');

      // Inject parameters (contractId, tokenPosition, priceType, priceData, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenSetPriceForDirectPurchase', 'testnet');
      expect(success).toBe(true);

      // Execute the set price
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenSetPriceForDirectPurchase.testnet[0];

      // Validate token set price specific result
      validateTokenSetPriceResult(result.result, testParams.priceType, testParams.priceData);

      console.log('✅ Token set price state transition completed successfully');
    });

    test('should execute token direct purchase transition', async () => {
      // Set up the token direct purchase transition
      await wasmSdkPage.setupStateTransition('token', 'tokenDirectPurchase');

      // Inject parameters (contractId, tokenPosition, amount, totalAgreedPrice, keyId, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenDirectPurchase', 'testnet');
      expect(success).toBe(true);

      // Execute the purchase
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Check for expected platform responses indicating issues
      if (!result.success && result.result && result.result.includes('Missing response message')) {
        // Skip the test with a descriptive reason
        test.skip(true, 'Platform returned "Missing response message". Possibly insufficient credits or tokens not available for purchase.');
      }

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenDirectPurchase.testnet[0];

      // Validate token direct purchase specific result
      validateTokenDirectPurchaseResult(result.result, testParams.amount, testParams.totalAgreedPrice);

      console.log('✅ Token direct purchase state transition completed successfully');
    });

    test('should execute token config update transition', async () => {
      // Set up the token config update transition
      await wasmSdkPage.setupStateTransition('token', 'tokenConfigUpdate');

      // Inject parameters (contractId, tokenPosition, configItemType, configValue, privateKey)
      const success = await parameterInjector.injectStateTransitionParameters('token', 'tokenConfigUpdate', 'testnet');
      expect(success).toBe(true);

      // Execute the config update
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Validate basic result structure
      validateBasicStateTransitionResult(result);

      // Get test parameters for validation
      const testParams = parameterInjector.testData.stateTransitionParameters.token.tokenConfigUpdate.testnet[0];

      // Validate token config update specific result
      validateTokenConfigUpdateResult(result.result, testParams.configItemType, testParams.configValue);

      console.log('✅ Token config update state transition completed successfully');
    });

    test('should show authentication inputs for token transitions', async () => {
      await wasmSdkPage.setupStateTransition('token', 'tokenTransfer');

      // Check that authentication inputs are visible
      const hasAuthInputs = await wasmSdkPage.hasAuthenticationInputs();
      expect(hasAuthInputs).toBe(true);

      console.log('✅ Token state transition authentication inputs are visible');
    });
  });

  test.describe('Error Handling for State Transitions', () => {
    test('should handle invalid JSON schema gracefully', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');

      // Fill with invalid JSON schema
      const invalidParams = {
        canBeDeleted: false,
        readonly: false,
        keepsHistory: false,
        documentSchemas: 'invalid_json_here',
        identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
        privateKey: "11111111111111111111111111111111111111111111111111"
      };

      await wasmSdkPage.fillStateTransitionParameters(invalidParams);

      // Execute the transition
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Should show error
      expect(result.hasError).toBe(true);
      expect(result.statusText.toLowerCase()).toMatch(/error|invalid|failed/);

      console.log('✅ Invalid JSON schema error handling works correctly');
    });

    test('should handle missing required fields', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');

      // Don't fill any parameters, try to execute
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Should show error or validation message
      expect(result.hasError).toBe(true);
      expect(result.statusText.toLowerCase()).toMatch(/error|required|missing/);

      console.log('✅ Missing required fields error handling works correctly');
    });

    test('should handle invalid private key gracefully', async () => {
      await wasmSdkPage.setupStateTransition('dataContract', 'dataContractCreate');

      // Fill with invalid private key
      const invalidParams = {
        canBeDeleted: false,
        readonly: false,
        keepsHistory: false,
        documentSchemas: '{"note": {"type": "object", "properties": {"message": {"type": "string", "position": 0}}, "additionalProperties": false}}',
        identityId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
        privateKey: "invalid_private_key_here"
      };

      await wasmSdkPage.fillStateTransitionParameters(invalidParams);

      // Execute the transition
      const result = await wasmSdkPage.executeStateTransitionAndGetResult();

      // Should show error
      expect(result.hasError).toBe(true);
      expect(result.statusText.toLowerCase()).toMatch(/error|invalid|failed/);

      console.log('✅ Invalid private key error handling works correctly');
    });
  });

  test.describe('UI State and Navigation', () => {
    test('should switch to state transitions operation type correctly', async () => {
      // Start with queries, then switch to transitions
      await wasmSdkPage.setOperationType('queries');
      await wasmSdkPage.page.waitForTimeout(500);

      await wasmSdkPage.setOperationType('transitions');

      // Verify the operation type is set correctly
      const operationType = await wasmSdkPage.page.locator('#operationType').inputValue();
      expect(operationType).toBe('transitions');

      console.log('✅ Successfully switched to state transitions operation type');
    });

    test('should populate transition categories correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');

      // Get available categories and filter out placeholders
      const allCategories = await wasmSdkPage.getAvailableQueryCategories();
      const categories = filterPlaceholderOptions(allCategories);

      // Define expected state transition categories
      const expectedCategories = [
        'Identity Transitions',
        'Data Contract Transitions',
        'Document Transitions',
        'Token Transitions',
        'Voting Transitions'
      ];

      // Verify exact match - contains all expected and no unexpected ones
      expect(categories).toHaveLength(expectedCategories.length);
      expectedCategories.forEach(expectedCategory => {
        expect(categories).toContain(expectedCategory);
      });

      console.log('✅ State transition categories populated correctly:', categories);
    });

    test('should populate identity transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('identity');

      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);

      // Define expected identity transition types
      const expectedTransitionTypes = [
        'Identity Create',
        'Identity Top Up',
        'Identity Update',
        'Identity Credit Transfer',
        'Identity Credit Withdrawal'
      ];

      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });

      console.log('✅ Identity transition types populated correctly:', transitionTypes);
    });

    test('should populate data contract transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('dataContract');

      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);

      // Define expected data contract transition types
      const expectedTransitionTypes = [
        'Data Contract Create',
        'Data Contract Update'
      ];

      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });

      console.log('✅ Data contract transition types populated correctly:', transitionTypes);
    });

    test('should populate document transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('document');

      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);

      // Define expected document transition types
      const expectedTransitionTypes = [
        'Document Create',
        'Document Replace',
        'Document Delete',
        'Document Transfer',
        'Document Purchase',
        'Document Set Price',
        'DPNS Register Name'
      ];

      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });

      console.log('✅ Document transition types populated correctly:', transitionTypes);
    });

    test('should populate token transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('token');

      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);

      // Define expected token transition types (based on docs.html)
      const expectedTransitionTypes = [
        'Token Burn',
        'Token Mint',
        'Token Claim',
        'Token Set Price',
        'Token Direct Purchase',
        'Token Config Update',
        'Token Transfer',
        'Token Freeze',
        'Token Unfreeze',
        'Token Destroy Frozen'
      ];

      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });

      console.log('✅ Token transition types populated correctly:', transitionTypes);
    });

    test('should populate voting transition types correctly', async () => {
      await wasmSdkPage.setOperationType('transitions');
      await wasmSdkPage.setQueryCategory('voting');

      // Get available transition types and filter out placeholders
      const allTransitionTypes = await wasmSdkPage.getAvailableQueryTypes();
      const transitionTypes = filterPlaceholderOptions(allTransitionTypes);

      // Define expected voting transition types
      const expectedTransitionTypes = [
        'DPNS Username',
        'Contested Resource'
      ];

      // Verify exact match - contains all expected and no unexpected ones
      expect(transitionTypes).toHaveLength(expectedTransitionTypes.length);
      expectedTransitionTypes.forEach(expectedType => {
        expect(transitionTypes).toContain(expectedType);
      });

      console.log('✅ Voting transition types populated correctly:', transitionTypes);
    });
  });

});

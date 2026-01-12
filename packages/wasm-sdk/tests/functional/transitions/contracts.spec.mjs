import init, * as sdk from '../../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from '../fixtures/requiredTestData.mjs';

/**
 * Contract state transition tests for wasm-sdk.
 *
 * Tests verify contract state transition methods work correctly against a local platform.
 * They require SDK_TEST_DATA=true when starting the platform to seed test identities.
 *
 * Test identities:
 * - Identity 1: 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi
 * - Identity 2: 8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR
 * - Identity 3: CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8
 */

describe('Contract State Transitions', function describeContractStateTransitions() {
  this.timeout(120000);

  let client;
  const testData = wasmFunctionalTestRequirements();

  // Store contract ID for use across tests
  let createdContractId = null;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('contractPublish', () => {
    it('creates a new data contract', async () => {
      // Contract operations require at least HIGH security level (key index 2)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // Create a simple test schema with a "note" document type
      // Position property is required for document types and their properties
      const schema = {
        note: {
          type: 'object',
          position: 0,
          properties: {
            message: {
              type: 'string',
              maxLength: 500,
              position: 0,
            },
          },
          required: ['message'],
          additionalProperties: false,
        },
      };

      // Create the data contract
      // Note: We use nonce 0 as a placeholder - the actual contract ID will be
      // assigned by the SDK during publishing based on the current identity nonce
      const dataContract = new sdk.DataContract(
        testData.identityId, // owner ID
        0n, // placeholder nonce (SDK will assign actual ID during publish)
        schema, // document schemas
        undefined, // definitions (optional)
        undefined, // tokens (optional)
        true, // full validation
        undefined, // platform version (use default)
      );

      // Publish the contract and get the published version with actual ID
      const publishedContract = await client.contractPublish({
        dataContract,
        identityKey,
        signer,
      });
      createdContractId = publishedContract.id;

      // Wait for the contract to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Verify the contract is available by fetching it
      const fetchedContract = await client.getDataContract(createdContractId);
      expect(fetchedContract).to.exist();
      expect(fetchedContract.id.toString()).to.equal(createdContractId.toString());
    });
  });

  describe('contractUpdate', () => {
    it('updates an existing data contract', async () => {
      // Requires contract from previous test
      expect(createdContractId).to.exist();

      // Contract update requires CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      // Wait briefly to ensure the previous state transition is fully processed
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Force refresh the nonce cache to ensure we get the latest nonce from platform
      // This is needed because contractPublish just used a nonce and the cache is stale
      await client.refreshIdentityNonce(sdk.Identifier.fromBase58(testData.identityId));

      // Fetch the existing contract
      const existingContract = await client.getDataContract(createdContractId);
      expect(existingContract).to.exist();

      // Update the contract by adding a new document type
      // We need to increment the version and add a new document type
      const currentVersion = existingContract.version;

      // Get existing schemas and add a new one
      const existingSchemas = existingContract.getSchemas();
      const updatedSchemas = {
        ...existingSchemas,
        task: {
          type: 'object',
          position: 1,
          properties: {
            title: {
              type: 'string',
              maxLength: 100,
              position: 0,
            },
            completed: {
              type: 'boolean',
              position: 1,
            },
          },
          required: ['title'],
          additionalProperties: false,
        },
      };

      // Set updated schemas and increment version
      existingContract.setSchemas(
        updatedSchemas,
        undefined, // definitions
        true, // full validation
        undefined, // platform version
      );
      existingContract.version = currentVersion + 1;

      // Update the contract on platform
      await client.contractUpdate({
        dataContract: existingContract,
        identityKey,
        signer,
      });

      // Wait for the platform to process the update
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Verify the update by fetching the contract again
      const updatedContract = await client.getDataContract(createdContractId);
      expect(updatedContract.version).to.equal(currentVersion + 1);
    });
  });
});

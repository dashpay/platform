import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from '../fixtures/requiredTestData.ts';

/**
 * Document state transition tests for wasm-sdk.
 *
 * Tests verify document state transition methods work correctly against a local platform.
 * They require SDK_TEST_DATA=true when starting the platform to seed test identities and contracts.
 *
 * Test identities:
 * - Identity 1: 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi
 * - Identity 2: 8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR
 * - Identity 3: CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8
 *
 * DPNS Contract: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec
 *
 * For document replace/delete/transfer tests, we create a custom contract with
 * mutable/deletable/transferable document types since DPNS documents are immutable.
 */

describe('Document State Transitions', function describeDocumentStateTransitions() {
  this.timeout(180000);

  let client: sdk.WasmSdk;
  const testData = wasmFunctionalTestRequirements();

  // Store contract and document IDs for use across tests
  let testContractId = null;
  let createdDocumentId = null;
  let mutableDocumentId = null;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('documentCreate', () => {
    it('creates a new document', async () => {
      // Document operations require at least HIGH security level (key index 2) for signing
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // Create a DPNS preorder document for testing
      // Using hash of salted domain label
      const saltedDomainHash = `${Date.now()}`.padStart(64, '0');

      const document = new sdk.Document({
        properties: {
          saltedDomainHash: Uint8Array.from(
            saltedDomainHash.match(/.{2}/g).map((byte) => parseInt(byte, 16)),
          ),
        },
        documentTypeName: 'preorder',
        revision: 1,
        dataContractId: testData.dpnsContractId,
        ownerId: testData.identityId,
      });

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      createdDocumentId = document.id;
      expect(createdDocumentId).to.exist();
    });
  });

  describe('Custom contract for mutable documents', () => {
    it('creates a contract with mutable document types', async () => {
      // Contract operations require at least HIGH security level (key index 2)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // Create a schema with mutable, deletable, and transferable document types
      // Position property is required for document types and properties
      const schema = {
        // Mutable document type - can be updated
        mutableNote: {
          type: 'object',
          position: 0,
          documentsMutable: true,
          canBeDeleted: true,
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
        // Transferable document type - can change ownership
        // transferable: 1 = Always transferable (see Transferable enum in DPP)
        transferableItem: {
          type: 'object',
          position: 1,
          transferable: 1,
          documentsMutable: false,
          canBeDeleted: false,
          properties: {
            name: {
              type: 'string',
              maxLength: 100,
              position: 0,
            },
          },
          required: ['name'],
          additionalProperties: false,
        },
      };

      // Create the data contract
      // Note: We use nonce 0 as a placeholder - the actual contract ID will be
      // assigned by the SDK during publishing based on the current identity nonce
      const dataContract = new sdk.DataContract({
        ownerId: testData.identityId,
        identityNonce: 0n, // placeholder nonce (SDK will assign actual ID during publish)
        schemas: schema,
        fullValidation: true,
      });

      // Publish the contract and get the published version with actual ID
      const publishedContract = await client.contractPublish({
        dataContract,
        identityKey,
        signer,
      });
      testContractId = publishedContract.id;

      // Wait for the contract to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Verify the contract is available
      const fetchedContract = await client.getDataContract(testContractId);
      expect(fetchedContract).to.exist();
    });
  });

  describe('documentReplace', () => {
    it('replaces an existing document', async () => {
      // Requires contract from previous test
      expect(testContractId).to.exist();

      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // First, create a mutable document
      const document = new sdk.Document({
        properties: { message: 'Original message' },
        documentTypeName: 'mutableNote',
        revision: 1,
        dataContractId: testContractId,
        ownerId: testData.identityId,
      });

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      mutableDocumentId = document.id;
      expect(mutableDocumentId).to.exist();

      // Wait for the document to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Now replace the document with updated content
      // Increment revision to 2 for the update
      const updatedDocument = new sdk.Document({
        properties: { message: 'Updated message' },
        documentTypeName: 'mutableNote',
        revision: 2, // Revision 2 for replacement
        dataContractId: testContractId,
        ownerId: testData.identityId,
        id: mutableDocumentId, // Use the same document ID
      });

      await client.documentReplace({
        document: updatedDocument,
        identityKey,
        signer,
      });
    });
  });

  describe('documentDelete', () => {
    it('deletes a document', async () => {
      // Requires contract from previous test
      expect(testContractId).to.exist();

      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // First, create a document to delete
      const document = new sdk.Document({
        properties: { message: 'Document to be deleted' },
        documentTypeName: 'mutableNote',
        revision: 1,
        dataContractId: testContractId,
        ownerId: testData.identityId,
      });

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      const documentId = document.id;
      expect(documentId).to.exist();

      // Wait for the document to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Now delete the document using object format
      await client.documentDelete({
        document: {
          id: documentId,
          ownerId: testData.identityId,
          dataContractId: testContractId,
          documentTypeName: 'mutableNote',
        },
        identityKey,
        signer,
      });
    });
  });

  describe('documentTransfer', () => {
    it('transfers document ownership', async () => {
      // Requires contract from previous test
      expect(testContractId).to.exist();

      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // First, create a transferable document
      const document = new sdk.Document({
        properties: { name: 'Transferable Item' },
        documentTypeName: 'transferableItem',
        revision: 1,
        dataContractId: testContractId,
        ownerId: testData.identityId,
      });

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      const documentId = document.id;
      expect(documentId).to.exist();

      // Wait for the document to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Create document object for transfer (revision incremented)
      const documentForTransfer = new sdk.Document({
        properties: { name: 'Transferable Item' },
        documentTypeName: 'transferableItem',
        revision: 2, // Increment revision for transfer
        dataContractId: testContractId,
        ownerId: testData.identityId,
        id: documentId,
      });

      // Transfer the document to Identity 2
      await client.documentTransfer({
        document: documentForTransfer,
        recipientId: testData.identityId2,
        identityKey,
        signer,
      });
    });
  });
});

import init, * as sdk from '../../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from '../fixtures/requiredTestData.mjs';

/**
 * Document state transition tests for wasm-sdk.
 *
 * These tests verify that document state transition methods work correctly against a local platform.
 * They require SDK_TEST_DATA=true when starting the platform to seed test identities and contracts.
 *
 * Test identities:
 * - Identity 1: 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi
 * - Identity 2: 8pkwN93v4wTWzzJAJkBCVPQJHMGwNa9TYfJUW24sAq11
 * - Identity 3: CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8
 *
 * DPNS Contract: GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec
 *
 * For document replace/delete/transfer tests, we create a custom contract with
 * mutable/deletable/transferable document types since DPNS documents are immutable.
 */

describe('Document State Transitions', function describeDocumentStateTransitions() {
  this.timeout(180000);

  let client;
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

  describe('documentCreate', function describeDocumentCreate() {
    it('creates a new document', async function testDocumentCreate() {
      // Document operations require at least HIGH security level (key index 2) for signing
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // Create a DPNS preorder document for testing
      // Using hash of salted domain label
      const saltedDomainHash = `${Date.now()}`.padStart(64, '0');

      const document = new sdk.Document(
        {
          saltedDomainHash: Uint8Array.from(
            saltedDomainHash.match(/.{2}/g).map((byte) => parseInt(byte, 16)),
          ),
        },
        'preorder',
        BigInt(1), // Revision must be BigInt
        testData.dpnsContractId,
        testData.identityId,
        undefined, // Document ID will be generated
      );

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      createdDocumentId = document.id;
      expect(createdDocumentId).to.exist;
    });
  });

  describe('Custom contract for mutable documents', function describeCustomContract() {
    it('creates a contract with mutable document types', async function testCreateMutableContract() {
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
      testContractId = publishedContract.id;

      // Verify the contract is available
      const fetchedContract = await client.getDataContract(testContractId);
      expect(fetchedContract).to.exist;
    });
  });

  describe('documentReplace', function describeDocumentReplace() {
    it('replaces an existing document', async function testDocumentReplace() {
      // Requires contract from previous test
      expect(testContractId).to.exist;

      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // First, create a mutable document
      const document = new sdk.Document(
        { message: 'Original message' },
        'mutableNote',
        BigInt(1), // Revision 1 for creation
        testContractId,
        testData.identityId,
        undefined, // Document ID will be generated
      );

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      mutableDocumentId = document.id;
      expect(mutableDocumentId).to.exist;

      // Wait for the document to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Now replace the document with updated content
      // Increment revision to 2 for the update
      const updatedDocument = new sdk.Document(
        { message: 'Updated message' },
        'mutableNote',
        BigInt(2), // Revision 2 for replacement
        testContractId,
        testData.identityId,
        mutableDocumentId, // Use the same document ID
      );

      await client.documentReplace({
        document: updatedDocument,
        identityKey,
        signer,
      });

      // Document was replaced successfully if no error thrown
      expect(true).to.be.true;
    });
  });

  describe('documentDelete', function describeDocumentDelete() {
    it('deletes a document', async function testDocumentDelete() {
      // Requires contract from previous test
      expect(testContractId).to.exist;

      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // First, create a document to delete
      const document = new sdk.Document(
        { message: 'Document to be deleted' },
        'mutableNote',
        BigInt(1),
        testContractId,
        testData.identityId,
        undefined,
      );

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      const documentId = document.id;
      expect(documentId).to.exist;

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

      // Document was deleted successfully if no error thrown
      expect(true).to.be.true;
    });
  });

  describe('documentTransfer', function describeDocumentTransfer() {
    it('transfers document ownership', async function testDocumentTransfer() {
      // Requires contract from previous test
      expect(testContractId).to.exist;

      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // First, create a transferable document
      const document = new sdk.Document(
        { name: 'Transferable Item' },
        'transferableItem',
        BigInt(1),
        testContractId,
        testData.identityId,
        undefined,
      );

      await client.documentCreate({
        document,
        identityKey,
        signer,
      });

      const documentId = document.id;
      expect(documentId).to.exist;

      // Wait for the document to be indexed on platform
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Create document object for transfer (revision incremented)
      const documentForTransfer = new sdk.Document(
        { name: 'Transferable Item' },
        'transferableItem',
        BigInt(2), // Increment revision for transfer
        testContractId,
        testData.identityId,
        documentId,
      );

      // Transfer the document to Identity 2
      await client.documentTransfer({
        document: documentForTransfer,
        recipientId: testData.identityId2,
        identityKey,
        signer,
      });

      // Document was transferred successfully if no error thrown
      expect(true).to.be.true;
    });
  });
});

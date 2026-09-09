import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';
import { prefetchLocalReady } from '../helpers/trustedContext.ts';
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
  const waitForPlatform = async (ms = 2000) => new Promise((resolve) => { setTimeout(resolve, ms); });
  const getSingleTokenBalance = async (identityId: string, tokenId: string) => {
    const balances = await client.getIdentityTokenBalances(identityId, [tokenId]);
    return balances.get(tokenId);
  };
  // Token balance proofs are served by nodes whose view can briefly lag the block
  // that committed a state transition, so poll until the balance converges instead
  // of asserting on a single immediate (and racy) read.
  const expectTokenBalance = async (
    identityId: string,
    tokenId: string,
    expected: bigint,
    { timeoutMs = 30000, intervalMs = 500 } = {},
  ) => {
    const startedAt = Date.now();
    const deadline = startedAt + timeoutMs;
    let actual = await getSingleTokenBalance(identityId, tokenId);
    while (actual !== expected && Date.now() < deadline) {
      await new Promise((resolve) => { setTimeout(resolve, intervalMs); });
      actual = await getSingleTokenBalance(identityId, tokenId);
    }
    // A timeout here is the most likely failure mode, so surface the triage data
    // (which identity/token, what we waited for, how long, and the last read).
    expect(actual).to.equal(
      expected,
      `token ${tokenId} balance for identity ${identityId} did not converge to ${expected} within ${Date.now() - startedAt}ms (last read: ${actual})`,
    );
  };
  const buildSimpleTokenConfiguration = (baseSupply: bigint, newTokensDestinationIdentity: string) => {
    const contractOwner = sdk.AuthorizedActionTakers.ContractOwner();
    const contractOwnerChangeRules = new sdk.ChangeControlRules({
      authorizedToMakeChange: contractOwner,
      adminActionTakers: contractOwner,
      isChangingAuthorizedActionTakersToNoOneAllowed: true,
      isChangingAdminActionTakersToNoOneAllowed: true,
      isSelfChangingAdminActionTakersAllowed: true,
    });

    return new sdk.TokenConfiguration({
      conventions: new sdk.TokenConfigurationConvention({
        en: new sdk.TokenConfigurationLocalization(false, 'ticket', 'tickets'),
      }, 0),
      conventionsChangeRules: contractOwnerChangeRules,
      baseSupply,
      keepsHistory: new sdk.TokenKeepsHistoryRules({}),
      maxSupplyChangeRules: contractOwnerChangeRules,
      distributionRules: new sdk.TokenDistributionRules({
        perpetualDistributionRules: contractOwnerChangeRules,
        newTokensDestinationIdentity,
        newTokensDestinationIdentityRules: contractOwnerChangeRules,
        mintingAllowChoosingDestination: false,
        mintingAllowChoosingDestinationRules: contractOwnerChangeRules,
        changeDirectPurchasePricingRules: contractOwnerChangeRules,
      }),
      marketplaceRules: new sdk.TokenMarketplaceRules(
        sdk.TokenTradeMode.NotTradeable(),
        contractOwnerChangeRules,
      ),
      manualMintingRules: contractOwnerChangeRules,
      manualBurningRules: contractOwnerChangeRules,
      freezeRules: contractOwnerChangeRules,
      unfreezeRules: contractOwnerChangeRules,
      destroyFrozenFundsRules: contractOwnerChangeRules,
      emergencyActionRules: contractOwnerChangeRules,
      mainControlGroupCanBeModified: sdk.AuthorizedActionTakers.NoOne(),
      description: 'token-paid document flow test token',
    });
  };
  const makeTokenPaymentInfo = (
    maximumTokenCost: bigint,
    overrides: Record<string, unknown> = {},
  ) => ({
    tokenContractPosition: 0,
    maximumTokenCost,
    gasFeesPaidBy: 'DocumentOwner',
    ...overrides,
  });

  // Store contract and document IDs for use across tests
  let testContractId = null;
  let createdDocumentId = null;
  let mutableDocumentId = null;
  let tokenPaidContractId = null;
  let tokenPaidDocumentId = null;
  let tokenPaidTokenId = null;

  before(async () => {
    await init();
    const context = await prefetchLocalReady();
    const builder = sdk.WasmSdkBuilder.local().withTrustedContext(context);
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('documentCreate()', () => {
    it('should create a new document', async () => {
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
    it('should create a contract with mutable document types', async () => {
      // Contract operations require at least HIGH security level (key index 2)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // Create a schema with mutable, deletable, and transferable document types.
      // `position` is required on each *property* (to order fields in the
      // document row) — it is NOT a valid key on the document-type root.
      // Under protocol v12 the document meta-schema is strict
      // (`additionalProperties: false`) and rejects stray root-level keys.
      const schema = {
        // Mutable document type - can be updated
        mutableNote: {
          type: 'object',
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
      await waitForPlatform();

      // Verify the contract is available
      const fetchedContract = await client.getDataContract(testContractId);
      expect(fetchedContract).to.exist();
    });
  });

  describe('documentReplace()', () => {
    it('should replace an existing document', async () => {
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
      await waitForPlatform();

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

  describe('documentDelete()', () => {
    it('should delete a document', async () => {
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
      await waitForPlatform();

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

  describe('documentTransfer()', () => {
    it('should transfer document ownership', async () => {
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
      await waitForPlatform();

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

  describe('tokenPaymentInfo document flow', () => {
    it('should publish a contract with document token costs and fund the seller and buyer', async () => {
      const { signer: contractSigner, identityKey: contractIdentityKey } = createTestSignerAndKey(sdk, 1, 2);
      const { signer: tokenSigner, identityKey: tokenIdentityKey } = createTestSignerAndKey(sdk, 1, 1);

      const schema = {
        tokenPaidListing: {
          type: 'object',
          documentsMutable: true,
          canBeDeleted: true,
          transferable: 1,
          tradeMode: 1,
          tokenCost: {
            create: { tokenPosition: 0, amount: 5, gasFeesPaidBy: 0 },
            replace: { tokenPosition: 0, amount: 4, gasFeesPaidBy: 0 },
            delete: { tokenPosition: 0, amount: 1, gasFeesPaidBy: 0 },
            transfer: { tokenPosition: 0, amount: 2, gasFeesPaidBy: 0 },
            update_price: { tokenPosition: 0, amount: 2, gasFeesPaidBy: 0 },
            purchase: { tokenPosition: 0, amount: 3, gasFeesPaidBy: 0 },
          },
          properties: {
            title: {
              type: 'string',
              maxLength: 100,
              position: 0,
            },
          },
          required: ['title'],
          additionalProperties: false,
        },
      };

      const tokens = {
        0: buildSimpleTokenConfiguration(1000n, testData.identityId),
      };

      const dataContract = new sdk.DataContract({
        ownerId: testData.identityId,
        identityNonce: 0n,
        schemas: schema,
        tokens,
        fullValidation: true,
      });

      const publishedContract = await client.contractPublish({
        dataContract,
        identityKey: contractIdentityKey,
        signer: contractSigner,
      });

      tokenPaidContractId = publishedContract.id;
      tokenPaidTokenId = sdk.WasmSdk.calculateTokenIdFromContract(tokenPaidContractId, 0);

      await waitForPlatform();

      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 1000n);

      await client.tokenTransfer({
        dataContractId: tokenPaidContractId,
        tokenPosition: 0,
        senderId: testData.identityId,
        recipientId: testData.identityId2,
        amount: 50n,
        identityKey: tokenIdentityKey,
        signer: tokenSigner,
      });

      await client.tokenTransfer({
        dataContractId: tokenPaidContractId,
        tokenPosition: 0,
        senderId: testData.identityId,
        recipientId: testData.identityId3,
        amount: 50n,
        identityKey: tokenIdentityKey,
        signer: tokenSigner,
      });

      await waitForPlatform();

      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 900n);
      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 50n);
      await expectTokenBalance(testData.identityId3, tokenPaidTokenId, 50n);
    });

    it('should reject create when tokenPaymentInfo is omitted', async () => {
      expect(tokenPaidContractId).to.exist();

      const { signer, identityKey } = createTestSignerAndKey(sdk, 2, 2);
      const document = new sdk.Document({
        properties: { title: `Missing token payment ${Date.now()}` },
        documentTypeName: 'tokenPaidListing',
        revision: 1,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
      });

      await expect(client.documentCreate({
        document,
        identityKey,
        signer,
      })).to.be.rejectedWith('Required token payment info not set');
    });

    it('should reject create when maximumTokenCost is below the required amount', async () => {
      expect(tokenPaidContractId).to.exist();

      const { signer, identityKey } = createTestSignerAndKey(sdk, 2, 2);
      const document = new sdk.Document({
        properties: { title: `Low token cap ${Date.now()}` },
        documentTypeName: 'tokenPaidListing',
        revision: 1,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
      });

      await expect(client.documentCreate({
        document,
        identityKey,
        signer,
        tokenPaymentInfo: makeTokenPaymentInfo(4n),
      })).to.be.rejectedWith('Identity has not agreed to pay the required token amount');
    });

    it('should reject create when paymentTokenContractId is explicitly set for an implicit token cost', async () => {
      expect(tokenPaidContractId).to.exist();

      const { signer, identityKey } = createTestSignerAndKey(sdk, 2, 2);
      const document = new sdk.Document({
        properties: { title: `Explicit payment token contract ${Date.now()}` },
        documentTypeName: 'tokenPaidListing',
        revision: 1,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
      });

      await expect(client.documentCreate({
        document,
        identityKey,
        signer,
        tokenPaymentInfo: makeTokenPaymentInfo(5n, {
          paymentTokenContractId: tokenPaidContractId,
        }),
      })).to.be.rejectedWith('Identity is trying to pay with the wrong token');
    });

    it('should replace, transfer, and delete documents with tokenPaymentInfo', async () => {
      expect(tokenPaidContractId).to.exist();
      expect(tokenPaidTokenId).to.exist();

      const { signer: sellerDocSigner, identityKey: sellerDocKey } = createTestSignerAndKey(sdk, 2, 2);

      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 900n);
      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 50n);
      await expectTokenBalance(testData.identityId3, tokenPaidTokenId, 50n);

      const updatableTitle = `Token paid mutable listing ${Date.now()}`;
      const updatableDocument = new sdk.Document({
        properties: { title: updatableTitle },
        documentTypeName: 'tokenPaidListing',
        revision: 1,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
      });

      await client.documentCreate({
        document: updatableDocument,
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(5n, {
          gasFeesPaidBy: 'documentOwner',
        }),
      });

      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 45n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 905n);

      await waitForPlatform();

      const replacedDocument = new sdk.Document({
        properties: { title: `${updatableTitle} updated` },
        documentTypeName: 'tokenPaidListing',
        revision: 2,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
        id: updatableDocument.id,
      });

      await client.documentReplace({
        document: replacedDocument,
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(4n, {
          gasFeesPaidBy: 0,
        }),
      });

      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 41n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 909n);

      await waitForPlatform();

      const transferableDocument = new sdk.Document({
        properties: { title: `${updatableTitle} updated` },
        documentTypeName: 'tokenPaidListing',
        revision: 3,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
        id: updatableDocument.id,
      });

      await client.documentTransfer({
        document: transferableDocument,
        recipientId: testData.identityId3,
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(2n),
      });

      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 39n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 911n);

      await waitForPlatform();

      const transferredDocument = await client.getDocument(
        tokenPaidContractId,
        'tokenPaidListing',
        updatableDocument.id,
      );

      expect(transferredDocument).to.exist();
      expect(transferredDocument.ownerId.toString()).to.equal(testData.identityId3);

      const deletableTitle = `Token paid deletable listing ${Date.now()}`;
      const deletableDocument = new sdk.Document({
        properties: { title: deletableTitle },
        documentTypeName: 'tokenPaidListing',
        revision: 1,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
      });

      await client.documentCreate({
        document: deletableDocument,
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(5n),
      });

      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 34n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 916n);

      await waitForPlatform();

      await client.documentDelete({
        document: {
          id: deletableDocument.id,
          ownerId: testData.identityId2,
          dataContractId: tokenPaidContractId,
          documentTypeName: 'tokenPaidListing',
        },
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(1n),
      });

      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 33n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 917n);
    });

    it('should create, price, and purchase a document with tokenPaymentInfo', async () => {
      expect(tokenPaidContractId).to.exist();
      expect(tokenPaidTokenId).to.exist();

      const { signer: sellerDocSigner, identityKey: sellerDocKey } = createTestSignerAndKey(sdk, 2, 2);
      const { signer: buyerDocSigner, identityKey: buyerDocKey } = createTestSignerAndKey(sdk, 3, 2);

      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 917n);
      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 33n);
      await expectTokenBalance(testData.identityId3, tokenPaidTokenId, 50n);

      const listingTitle = `Token paid listing ${Date.now()}`;
      const document = new sdk.Document({
        properties: { title: listingTitle },
        documentTypeName: 'tokenPaidListing',
        revision: 1,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
      });

      await client.documentCreate({
        document,
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(5n),
      });

      tokenPaidDocumentId = document.id;
      expect(tokenPaidDocumentId).to.exist();
      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 28n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 922n);

      await waitForPlatform();

      const documentForSale = new sdk.Document({
        properties: { title: listingTitle },
        documentTypeName: 'tokenPaidListing',
        revision: 2,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
        id: tokenPaidDocumentId,
      });

      await client.documentSetPrice({
        document: documentForSale,
        price: 1_000_000n,
        identityKey: sellerDocKey,
        signer: sellerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(2n),
      });

      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 26n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 924n);

      await waitForPlatform();

      const documentToPurchase = new sdk.Document({
        properties: { title: listingTitle },
        documentTypeName: 'tokenPaidListing',
        revision: 3,
        dataContractId: tokenPaidContractId,
        ownerId: testData.identityId2,
        id: tokenPaidDocumentId,
      });

      await client.documentPurchase({
        document: documentToPurchase,
        buyerId: testData.identityId3,
        price: 1_000_000n,
        identityKey: buyerDocKey,
        signer: buyerDocSigner,
        tokenPaymentInfo: makeTokenPaymentInfo(3n),
      });

      const purchaseDeadline = Date.now() + 30000;
      let purchasedDocument = await client.getDocument(
        tokenPaidContractId,
        'tokenPaidListing',
        tokenPaidDocumentId,
      );
      while (
        purchasedDocument?.ownerId.toString() !== testData.identityId3
        && Date.now() < purchaseDeadline
      ) {
        await new Promise((resolve) => { setTimeout(resolve, 500); });
        purchasedDocument = await client.getDocument(
          tokenPaidContractId,
          'tokenPaidListing',
          tokenPaidDocumentId,
        );
      }

      expect(purchasedDocument).to.exist();
      expect(purchasedDocument.ownerId.toString()).to.equal(
        testData.identityId3,
        `owner did not converge in 30s; last owner: ${purchasedDocument.ownerId}`,
      );
      await expectTokenBalance(testData.identityId2, tokenPaidTokenId, 26n);
      await expectTokenBalance(testData.identityId3, tokenPaidTokenId, 47n);
      await expectTokenBalance(testData.identityId, tokenPaidTokenId, 927n);
    });
  });
});

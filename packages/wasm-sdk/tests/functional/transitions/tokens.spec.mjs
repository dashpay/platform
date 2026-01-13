import init, * as sdk from '../../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from '../fixtures/requiredTestData.mjs';

/**
 * Token state transition tests for wasm-sdk.
 *
 * These tests verify that token state transition methods work correctly against a local platform.
 * They require SDK_TEST_DATA=true when starting the platform to seed test identities and tokens.
 *
 * Test identities:
 * - Identity 1: 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi (owns token contract)
 * - Identity 2: 8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR (has tokens, frozen on TOKEN_0)
 * - Identity 3: CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8 (has tokens)
 *
 * Token contract: CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8
 * - Token 0: has history tracking, has group minting (Group 0 - Identity 1 and 2)
 * - Token 1: paused
 * - Token 2: has group burning (Group 2 - Identity 1 and 3, power=1)
 *
 * Authorization (all via ContractOwner = Identity 1):
 * - freeze/unfreeze: ContractOwner
 * - emergency action: ContractOwner
 * - set price: ContractOwner
 */

describe('Token State Transitions', function describeTokenStateTransitions() {
  this.timeout(120000);

  let client;
  const testData = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('tokenTransfer', () => {
    it('transfers tokens between identities', async () => {
      // Identity 1 transfers to Identity 3 (which exists in genesis state)
      // Identity 1 has 100 tokens on TOKEN_0
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      const result = await client.tokenTransfer({
        dataContractId: testData.tokenContracts[0].contractId,
        tokenPosition: testData.tokenContracts[0].position,
        senderId: testData.identityId,
        recipientId: testData.identityId3, // Transfer to Identity 3
        amount: 10n, // Must be bigint, not string
        identityKey,
        signer,
      });

      expect(result).to.exist();
      // Token 0 has history tracking, so we get a document back instead of balances
      expect(result.document).to.exist();
      expect(result.document).to.be.instanceOf(sdk.Document);
    });
  });

  describe('tokenBurn', () => {
    it('burns tokens via group action', async () => {
      // Token 2 has group burning with Group 2 (Identity 1 and 3, required power = 1)
      // Identity 1 has power 1, so can burn alone as proposer
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      // Create group info for proposer (Group 2 for token burn)
      const groupInfo = sdk.GroupStateTransitionInfoStatus.proposer(2);

      const result = await client.tokenBurn({
        dataContractId: testData.tokenContracts[2].contractId,
        tokenPosition: testData.tokenContracts[2].position,
        identityId: testData.identityId,
        amount: 5n, // Must be bigint, not string
        identityKey,
        signer,
        groupInfo, // Required for group-managed burning
      });

      expect(result).to.exist();
      // Should return group power since this is a group action
      expect(result.groupPower).to.exist();
    });
  });

  describe('tokenMint', () => {
    it('mints tokens via group action', async () => {
      // Token 0 has group minting with Group 0 (Identity 1 and 2, required power = 1)
      // Identity 1 has power 1, so can mint alone as proposer
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      // Create group info for proposer (Group 0 for token mint)
      const groupInfo = sdk.GroupStateTransitionInfoStatus.proposer(0);

      const result = await client.tokenMint({
        dataContractId: testData.tokenContracts[0].contractId,
        tokenPosition: testData.tokenContracts[0].position,
        identityId: testData.identityId,
        recipientId: testData.identityId,
        amount: 100n, // Must be bigint, not string
        identityKey,
        signer,
        groupInfo, // Required for group-managed minting
      });

      expect(result).to.exist();
      // Should have group power since this is a group action
      expect(result.groupPower).to.exist();
    });
  });

  describe('tokenFreeze', () => {
    it('freezes an identity token balance', async () => {
      // Contract owner (Identity 1) can freeze tokens
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      const result = await client.tokenFreeze({
        dataContractId: testData.tokenContracts[0].contractId,
        tokenPosition: testData.tokenContracts[0].position,
        authorityId: testData.identityId,
        frozenIdentityId: testData.identityId3, // Freeze Identity 3
        identityKey,
        signer,
      });

      expect(result).to.exist();
    });
  });

  describe('tokenUnfreeze', () => {
    it('unfreezes an identity token balance', async () => {
      // Wait for previous freeze to be processed
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Identity 2 is already frozen on TOKEN_0 in genesis state
      // Contract owner (Identity 1) can unfreeze
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      const result = await client.tokenUnfreeze({
        dataContractId: testData.tokenContracts[0].contractId,
        tokenPosition: testData.tokenContracts[0].position,
        authorityId: testData.identityId,
        frozenIdentityId: testData.identityId3, // Unfreeze Identity 3 (frozen in previous test)
        identityKey,
        signer,
      });

      expect(result).to.exist();
    });
  });

  describe('tokenEmergencyAction', () => {
    it('pauses a token', async () => {
      // Contract owner (Identity 1) can pause/resume tokens
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      const result = await client.tokenEmergencyAction({
        dataContractId: testData.tokenContracts[0].contractId,
        tokenPosition: testData.tokenContracts[0].position,
        authorityId: testData.identityId,
        action: 'pause',
        identityKey,
        signer,
      });

      expect(result).to.exist();
    });

    it('resumes a paused token', async () => {
      // Wait for previous pause to be processed
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Resume Token 0 that was just paused
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      const result = await client.tokenEmergencyAction({
        dataContractId: testData.tokenContracts[0].contractId, // Token 0 was just paused
        tokenPosition: testData.tokenContracts[0].position,
        authorityId: testData.identityId,
        action: 'resume',
        identityKey,
        signer,
      });

      expect(result).to.exist();
    });
  });

  describe('tokenSetPrice', () => {
    it('sets a direct purchase price for tokens', async () => {
      // Contract owner (Identity 1) can set token prices
      // Token operations require CRITICAL security level (key index 1)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 1);

      const result = await client.tokenSetPrice({
        dataContractId: testData.tokenContracts[0].contractId,
        tokenPosition: testData.tokenContracts[0].position,
        authorityId: testData.identityId,
        price: 1000n, // Simple price field
        identityKey,
        signer,
      });

      expect(result).to.exist();
    });
  });
});

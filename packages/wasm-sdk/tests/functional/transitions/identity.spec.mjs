import init, * as sdk from '../../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from '../fixtures/requiredTestData.mjs';

/**
 * Identity state transition tests for wasm-sdk.
 *
 * Tests verify identity state transition methods work correctly against a local platform.
 * They require SDK_TEST_DATA=true when starting the platform to seed test identities with credits.
 *
 * Test identities:
 * - Identity 1: 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi (100 DASH worth of credits)
 * - Identity 2: 8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR
 * - Identity 3: CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8
 *
 * Key indices:
 * - 0: MASTER level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 1: CRITICAL level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 2: HIGH level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 3: CRITICAL level TRANSFER key (ECDSA_HASH160) - for credit transfers
 */

describe('Identity State Transitions', function describeIdentityStateTransitions() {
  this.timeout(60000);

  let client;
  const testData = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(async () => {
    // Wait a bit for any pending operations to settle before freeing
    await new Promise((resolve) => { setTimeout(resolve, 100); });
    if (client) {
      try {
        client.free();
      } catch (err) {
        // Ignore errors from double-free or borrow issues
        // These can happen if a test timed out with pending operations
        // eslint-disable-next-line no-console
        console.log('Note: client.free() error (may be harmless):', err.message);
      }
    }
  });

  describe('identityCreditTransfer', () => {
    it('transfers credits between identities', async () => {
      // Identity 1 transfers credits to Identity 2
      // Key index 3 is the TRANSFER purpose key (ECDSA_HASH160)
      const { signer } = createTestSignerAndKey(sdk, 1, 3);

      // Fetch the sender identity
      const identity = await client.getIdentity(testData.identityId);

      // Force refresh the nonce cache to ensure we get the latest nonce from platform
      // This is needed because other tests may have used nonces and the cache is stale
      await client.refreshIdentityNonce(sdk.Identifier.fromBase58(testData.identityId));

      const result = await client.identityCreditTransfer({
        identity,
        recipientId: testData.identityId2,
        amount: 100000n,
        signer,
      });

      expect(result).to.exist();
      expect(result.senderBalance).to.exist();
      expect(result.recipientBalance).to.exist();
    });
  });

  describe('identityUpdate', () => {
    it('adds a new public key to identity', async () => {
      // Identity update requires MASTER key (key index 0) for signing the transition
      const { signer } = createTestSignerAndKey(sdk, 1, 0);

      // Fetch the identity
      const identity = await client.getIdentity(testData.identityId);
      expect(identity).to.exist();

      // Get the current number of public keys
      const publicKeysBefore = identity.getPublicKeys();
      const keyCountBefore = publicKeysBefore.length;

      // Generate a new random key pair for the new public key
      const newKeyPair = sdk.WasmSdk.generateKeyPair('testnet');
      const newPublicKeyData = Uint8Array.from(
        newKeyPair.publicKey.match(/.{2}/g).map((byte) => parseInt(byte, 16)),
      );

      // IMPORTANT: The signer must also have the new key's private key
      // so it can sign the key proof for the new key being added
      const newPrivateKey = sdk.PrivateKey.fromHex(newKeyPair.privateKeyHex, 'testnet');
      signer.addKey(newPrivateKey);

      // Create a new public key to add
      const newPublicKeyInCreation = new sdk.IdentityPublicKeyInCreation(
        0, // keyId - will be overwritten by SDK to next available
        'AUTHENTICATION',
        'MEDIUM', // MEDIUM so it can be disabled later if needed
        'ECDSA_SECP256K1',
        false, // read only
        newPublicKeyData,
        undefined, // signature
        undefined, // contract bounds
      );

      await client.identityUpdate({
        identity,
        addPublicKeys: [newPublicKeyInCreation],
        signer,
      });

      // Wait for the platform to process the update
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Verify the update by fetching the identity again
      const updatedIdentity = await client.getIdentity(testData.identityId);
      const publicKeysAfter = updatedIdentity.getPublicKeys();
      expect(publicKeysAfter.length).to.equal(keyCountBefore + 1);
    });

    it('disables a public key on identity', async () => {
      // Identity update requires MASTER key (key index 0)
      const { signer } = createTestSignerAndKey(sdk, 1, 0);

      // Fetch the identity
      const identity = await client.getIdentity(testData.identityId);
      expect(identity).to.exist();

      // Find a key that can be disabled (MEDIUM or lower security level, not master)
      const publicKeys = identity.getPublicKeys();
      const keyToDisable = publicKeys.find(
        (key) => key.securityLevel === 'MEDIUM' && key.purpose === 'AUTHENTICATION',
      );

      // Requires a MEDIUM key from previous test
      expect(keyToDisable).to.exist();

      const keyIdToDisable = keyToDisable.keyId;

      await client.identityUpdate({
        identity,
        disablePublicKeys: [keyIdToDisable],
        signer,
      });

      // Verify the key was disabled by fetching the identity again
      const updatedIdentity = await client.getIdentity(testData.identityId);
      const disabledKey = updatedIdentity.getPublicKeys()
        .find((key) => key.keyId === keyIdToDisable);

      // The key should still exist but have a disabledAt timestamp
      expect(disabledKey).to.exist();
      expect(disabledKey.disabledAt).to.exist();
    });
  });

  describe('identityWithdrawal', () => {
    it('withdraws credits from platform', async () => {
      // Use the TRANSFER key (index 3) for withdrawal
      const { signer } = createTestSignerAndKey(sdk, 1, 3);

      // Get the identity
      const identity = await client.getIdentity(testData.identityId);
      expect(identity).to.exist();

      // Refresh nonce to ensure we have the latest
      await client.refreshIdentityNonce(sdk.Identifier.fromBase58(testData.identityId));

      // Small delay to avoid nonce race conditions in rapid test runs
      await new Promise((resolve) => { setTimeout(resolve, 500); });

      // Withdraw credits - not specifying toAddress means withdrawal
      // will be to the identity's registered withdrawal address
      // Minimum is 190000 credits, maximum is 50000000000000
      const remainingBalance = await client.identityCreditWithdrawal({
        identity,
        amount: 200000n, // Must be >= 190000
        coreFeePerByte: 1,
        signer,
      });

      expect(remainingBalance).to.exist();
    });
  });

  describe('identityTopUp', () => {
    it.skip('tops up identity balance', async () => {
      // This test requires an asset lock proof which needs:
      // 1. A funded wallet with Dash
      // 2. Creating an asset lock transaction on core chain
      // 3. Waiting for instant send lock or chain confirmation
      // This is complex to set up in functional tests
    });
  });

  describe('identityCreate', () => {
    it.skip('creates a new identity', async () => {
      // This test requires an asset lock proof from core blockchain
      // Similar complexity to identityTopUp
    });
  });
});

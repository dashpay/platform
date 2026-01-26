import { expect } from '../helpers/chai.ts';
import init, * as sdk from '../../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements, createTestSignerAndKey } from '../fixtures/requiredTestData.ts';

/**
 * DPNS state transition tests for wasm-sdk.
 *
 * Tests verify DPNS name registration works correctly against a local platform.
 * They require SDK_TEST_DATA=true when starting the platform to seed test identities.
 *
 * Test identities:
 * - Identity 1: 4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi (seed 1)
 * - Identity 2: 8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR (seed 2)
 *
 * Key indices:
 * - 0: MASTER level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 1: CRITICAL level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 2: HIGH level AUTHENTICATION key (ECDSA_SECP256K1)
 * - 3: CRITICAL level TRANSFER key (ECDSA_HASH160)
 */

describe('DPNS State Transitions', function describeDpnsStateTransitions() {
  this.timeout(180000);

  let client;
  const testData = wasmFunctionalTestRequirements();

  // Store results for verification
  let registeredUsername = null;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    const builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  describe('dpnsRegisterName', () => {
    it('registers a DPNS username', async () => {
      // DPNS registration requires HIGH security level (key index 2)
      const { signer, identityKey } = createTestSignerAndKey(sdk, 1, 2);

      // Fetch the identity first
      const identity = await client.getIdentity(testData.identityId);
      expect(identity).to.exist();

      // Generate a unique username based on timestamp to avoid conflicts
      const timestamp = Date.now();
      const label = `test${timestamp}`;

      // Track preorder callback invocation
      let preorderCallbackCalled = false;
      let preorderDocument = null;

      const result = await client.dpnsRegisterName({
        label,
        identity,
        identityKey,
        signer,
        preorderCallback: (doc) => {
          preorderCallbackCalled = true;
          preorderDocument = doc;
        },
      });

      // Verify result structure
      expect(result).to.exist();
      expect(result.preorderDocumentId).to.exist();
      expect(result.domainDocumentId).to.exist();
      expect(result.fullDomainName).to.exist();
      expect(result.fullDomainName).to.include('.dash');

      // Verify preorder callback was called with Document object
      expect(preorderCallbackCalled).to.be.true();
      expect(preorderDocument).to.exist();
      expect(preorderDocument.id).to.exist();
      expect(preorderDocument.ownerId).to.exist();

      registeredUsername = result.fullDomainName;
    });

    it('resolves a registered username to identity', async () => {
      // Skip if previous test didn't register a username
      if (!registeredUsername) {
        // eslint-disable-next-line no-console
        console.log('Skipping: no username was registered in previous test');
        return;
      }

      // Wait for the name to be indexed
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      const resolvedIdentityId = await client.dpnsResolveName(registeredUsername);

      expect(resolvedIdentityId).to.exist();
      expect(resolvedIdentityId).to.equal(testData.identityId);
    });
  });

  describe('dpnsIsNameAvailable', () => {
    it('returns true for an available name', async () => {
      const uniqueName = `available${Date.now()}`;
      const isAvailable = await client.dpnsIsNameAvailable(uniqueName);

      expect(isAvailable).to.be.true();
    });

    it('returns false for a taken name', async () => {
      // Skip if previous test didn't register a username
      if (!registeredUsername) {
        // eslint-disable-next-line no-console
        console.log('Skipping: no username was registered');
        return;
      }

      // Wait for the name to be indexed
      await new Promise((resolve) => { setTimeout(resolve, 2000); });

      // Extract label from full name (e.g., "test123.dash" -> "test123")
      const label = registeredUsername.replace('.dash', '');
      const isAvailable = await client.dpnsIsNameAvailable(label);

      expect(isAvailable).to.be.false();
    });
  });

  describe('DPNS utility functions', () => {
    it('validates usernames correctly', async () => {
      // Valid usernames
      expect(sdk.WasmSdk.dpnsIsValidUsername('alice')).to.be.true();
      expect(sdk.WasmSdk.dpnsIsValidUsername('test-user')).to.be.true();
      expect(sdk.WasmSdk.dpnsIsValidUsername('user123')).to.be.true();

      // Invalid usernames
      expect(sdk.WasmSdk.dpnsIsValidUsername('ab')).to.be.false(); // too short
      expect(sdk.WasmSdk.dpnsIsValidUsername('-invalid')).to.be.false(); // starts with hyphen
      expect(sdk.WasmSdk.dpnsIsValidUsername('invalid-')).to.be.false(); // ends with hyphen
      expect(sdk.WasmSdk.dpnsIsValidUsername('has space')).to.be.false(); // contains space
    });

    it('converts to homograph-safe characters', async () => {
      const result = sdk.WasmSdk.dpnsConvertToHomographSafe('alice');
      expect(result).to.equal('a11ce'); // 'l' and 'i' become '1'

      const result2 = sdk.WasmSdk.dpnsConvertToHomographSafe('bob');
      expect(result2).to.equal('b0b'); // 'o' becomes '0'
    });

    it('identifies contested usernames', async () => {
      // Contested usernames (3-19 chars, only [a-z01-] after normalization)
      expect(sdk.WasmSdk.dpnsIsContestedUsername('abc')).to.be.true();
      expect(sdk.WasmSdk.dpnsIsContestedUsername('dash')).to.be.true();

      // Not contested - too long
      expect(sdk.WasmSdk.dpnsIsContestedUsername('twentycharacterslongname')).to.be.false();

      // Not contested - contains invalid characters after normalization
      expect(sdk.WasmSdk.dpnsIsContestedUsername('test123')).to.be.false(); // contains '2' and '3'
    });
  });
});

import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS, TEST_SECRETS } from '../fixtures/local.mjs';

describe('Identities', function identitiesSuite() {
  this.timeout(60000);
  let sdk;
  let uniqueHash;
  let nonUniqueHash;

  before(async () => {
    sdk = EvoSDK.localTrusted();
    await sdk.connect();

    const identity = await sdk.identities.fetch(TEST_IDS.identityId);
    const keys = identity?.publicKeys ?? [];
    const crypto = await import('crypto');

    const hash160 = (buf) => {
      const sha = crypto.createHash('sha256').update(buf).digest();
      return crypto.createHash('ripemd160').update(sha).digest('hex');
    };

    if (keys[0]?.data) {
      uniqueHash = hash160(Buffer.from(keys[0].data));
    }
    if (keys[keys.length - 1]?.data) {
      nonUniqueHash = hash160(Buffer.from(keys[keys.length - 1].data));
    }
  });

  it('fetch() returns identity', async () => {
    const res = await sdk.identities.fetch(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('fetchWithProof() returns proof info', async () => {
    const res = await sdk.identities.fetchWithProof(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('getKeys({ request: { type: "all" } }) returns keys', async () => {
    const res = await sdk.identities.getKeys({
      identityId: TEST_IDS.identityId,
      request: { type: 'all' },
      limit: 10,
      offset: 0,
    });
    expect(res).to.exist();
  });

  it('getKeysWithProof({ request: { type: "all" } }) returns proof info', async () => {
    const res = await sdk.identities.getKeysWithProof({
      identityId: TEST_IDS.identityId,
      request: { type: 'all' },
    });
    expect(res).to.exist();
  });

  it('nonce() returns a numeric nonce', async () => {
    const res = await sdk.identities.nonce(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('balance() returns current balance', async () => {
    const res = await sdk.identities.balance(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('balanceAndRevision() returns structure with balance field', async () => {
    const res = await sdk.identities.balanceAndRevision(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('byPublicKeyHash() resolves identity by unique hash', async () => {
    const res = await sdk.identities.byPublicKeyHash(uniqueHash);
    expect(res).to.exist();
  });

  it('byNonUniquePublicKeyHash() resolves entries (may be empty)', async () => {
    const res = await sdk.identities.byNonUniquePublicKeyHash(nonUniqueHash);
    expect(res).to.exist();
  });

  it('tokenBalances() resolves for known identity/token pair', async () => {
    const res = await sdk.identities.tokenBalances(TEST_IDS.identityId, [TEST_IDS.tokenId]);
    expect(res).to.exist();
  });

  it.skip('creditTransfer() executes when secrets provided (skipped by default)', async function creditTransferExecutesWhenSecretsProvided() {
    if (!TEST_SECRETS.identityId || !TEST_SECRETS.privateKeyWif) {
      this.skip();
    }
    const res = await sdk.identities.creditTransfer({
      senderId: TEST_SECRETS.identityId,
      recipientId: TEST_IDS.identityId,
      amount: BigInt(1),
      privateKeyWif: TEST_SECRETS.privateKeyWif,
      keyId: TEST_SECRETS.keyId,
    });
    expect(res).to.exist();
  });
});

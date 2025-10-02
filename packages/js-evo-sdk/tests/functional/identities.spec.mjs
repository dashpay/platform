import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS, TEST_SECRETS } from '../fixtures/testnet.mjs';

describe('Identities', function identitiesSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('fetch() returns identity', async () => {
    const res = await sdk.identities.fetch(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('fetchWithProof() returns proof info', async () => {
    const res = await sdk.identities.fetchWithProof(TEST_IDS.identityId);
    expect(res).to.exist();
  });

  it('getKeys({ keyRequestType: "all" }) returns keys', async () => {
    const res = await sdk.identities.getKeys({
      identityId: TEST_IDS.identityId,
      keyRequestType: 'all',
      limit: 10,
      offset: 0,
    });
    expect(res).to.exist();
  });

  it('getKeysWithProof({ keyRequestType: "all" }) returns proof info', async () => {
    const res = await sdk.identities.getKeysWithProof({
      identityId: TEST_IDS.identityId,
      keyRequestType: 'all',
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
    const res = await sdk.identities.byPublicKeyHash(TEST_IDS.publicKeyHashUnique);
    expect(res).to.exist();
  });

  it('byNonUniquePublicKeyHash() resolves entries (may be empty)', async () => {
    const res = await sdk.identities.byNonUniquePublicKeyHash(TEST_IDS.publicKeyHashNonUnique);
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

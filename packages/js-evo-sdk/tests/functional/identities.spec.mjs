import { expect } from 'chai';
import { EvoSDK } from '../../dist/sdk.js';
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

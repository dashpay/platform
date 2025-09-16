import { expect } from 'chai';
import { EvoSDK } from '../../dist/sdk.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('DPNS', function () {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('isNameAvailable() returns a boolean', async () => {
    const res = await sdk.dpns.isNameAvailable('nonexistentname' + Math.floor(Math.random() * 1e6));
    expect(res).to.be.a('boolean');
  });

  it('resolveName() resolves known username', async () => {
    const res = await sdk.dpns.resolveName(TEST_IDS.username);
    expect(res).to.exist;
  });

  it('usernames() returns usernames for identity', async () => {
    const res = await sdk.dpns.usernames(TEST_IDS.identityId, { limit: 5 });
    expect(res).to.exist;
  });

  it('username() returns username for identity', async () => {
    const res = await sdk.dpns.username(TEST_IDS.identityId);
    expect(res).to.exist;
  });
});

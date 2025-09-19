import { expect } from 'chai';
import { EvoSDK } from '../../dist/sdk.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('System', function systemSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('status() returns basic node status', async () => {
    const res = await sdk.system.status();
    expect(res).to.exist();
  });

  it('currentQuorumsInfo() returns quorum info', async () => {
    const res = await sdk.system.currentQuorumsInfo();
    expect(res).to.exist();
  });

  it('totalCreditsInPlatform() returns value', async () => {
    const res = await sdk.system.totalCreditsInPlatform();
    expect(res).to.exist();
  });

  it('prefundedSpecializedBalance(identityId) returns value', async () => {
    const res = await sdk.system.prefundedSpecializedBalance(TEST_IDS.specializedBalanceIdentityId);
    expect(res).to.exist();
  });
});

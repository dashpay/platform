import { expect } from 'chai';
import { EvoSDK } from '../../dist/sdk.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Protocol', function () {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('versionUpgradeState() returns state', async () => {
    const res = await sdk.protocol.versionUpgradeState();
    expect(res).to.exist;
  });

  it('versionUpgradeVoteStatus() returns vote window status', async () => {
    const res = await sdk.protocol.versionUpgradeVoteStatus({ startProTxHash: TEST_IDS.proTxHash, count: 1 });
    expect(res).to.exist;
  });
});

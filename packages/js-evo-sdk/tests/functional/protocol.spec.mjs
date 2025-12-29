import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Protocol', function protocolSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('versionUpgradeState() returns state', async () => {
    const res = await sdk.protocol.versionUpgradeState();
    expect(res).to.exist();
  });

  it('versionUpgradeStateWithProof() returns state with proof', async () => {
    const res = await sdk.protocol.versionUpgradeStateWithProof();
    expect(res).to.exist();
    expect(res.data).to.exist();
    expect(res.proof).to.exist();
    expect(res.metadata).to.exist();
  });

  it('versionUpgradeVoteStatus() returns vote window status', async () => {
    const res = await sdk.protocol.versionUpgradeVoteStatus(TEST_IDS.proTxHash, 1);
    expect(res).to.exist();
  });

  it('versionUpgradeVoteStatusWithProof() returns vote status with proof', async () => {
    const res = await sdk.protocol.versionUpgradeVoteStatusWithProof(TEST_IDS.proTxHash, 50);
    expect(res).to.exist();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.exist();
    expect(res.metadata).to.exist();
  });
});

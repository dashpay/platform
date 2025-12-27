import { EvoSDK } from '../../dist/evo-sdk.module.js';
import { TEST_IDS } from '../fixtures/testnet.mjs';

describe('Epoch', function epochSuite() {
  this.timeout(60000);
  let sdk;

  before(async () => {
    sdk = EvoSDK.testnetTrusted();
    await sdk.connect();
  });

  it('current() returns current epoch', async () => {
    const res = await sdk.epoch.current();
    expect(res).to.exist();
  });

  it('epochsInfo() returns info for range', async () => {
    const res = await sdk.epoch.epochsInfo({ startEpoch: TEST_IDS.epoch, count: 1 });
    expect(res).to.exist();
  });

  it('evonodesProposedBlocksByRange() returns results', async () => {
    const res = await sdk.epoch.evonodesProposedBlocksByRange({ epoch: TEST_IDS.epoch, limit: 5 });
    expect(res).to.exist();
  });

  it('evonodesProposedBlocksByIdsWithProof() returns results with proof', async () => {
    const res = await sdk.epoch.evonodesProposedBlocksByIdsWithProof(TEST_IDS.epoch, [TEST_IDS.proTxHash]);
    expect(res).to.exist();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.exist();
    expect(res.metadata).to.exist();
  });

  it('evonodesProposedBlocksByRangeWithProof() returns results with proof', async () => {
    const res = await sdk.epoch.evonodesProposedBlocksByRangeWithProof({ epoch: TEST_IDS.epoch, limit: 5 });
    expect(res).to.exist();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.exist();
    expect(res.metadata).to.exist();
  });
});

import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Epochs and evonode blocks', function describeEpochs() {
  this.timeout(60000);

  let client;
  let builder;
  const { sampleEpoch, evonodeProTxHash } = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('gets epochs info and finalized epochs', async () => {
    if (!sampleEpoch) {
      this.skip();
    }
    const current = await client.getCurrentEpoch().catch(() => null);
    const currentIndex = current ? Number(current.index) : Number(sampleEpoch);
    const start = Math.max(0, currentIndex - 5);

    const infos = await client.getEpochsInfo({
      startEpoch: start,
      count: 5,
      ascending: true,
    });
    expect(infos).to.be.instanceOf(Map);

    const finalized = await client.getFinalizedEpochInfos({
      startEpoch: start,
      count: 5,
    });
    expect(finalized).to.be.instanceOf(Map);
  });

  it('queries evonode proposed blocks by id/range', async () => {
    if (!evonodeProTxHash) {
      this.skip();
    }
    const EVONODE_ID = evonodeProTxHash;
    await client.getEvonodesProposedEpochBlocksByIds(8635, [EVONODE_ID]);
    await client.getEvonodesProposedEpochBlocksByRange({
      epoch: sampleEpoch,
      startAfter: EVONODE_ID,
      limit: 50,
    });
  });
});

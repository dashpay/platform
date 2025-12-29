import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Epochs and evonode blocks', function describeEpochs() {
  this.timeout(60000);

  let client;
  let builder;
  const { sampleEpoch, evonodeProTxHash } = wasmFunctionalTestRequirements();

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsLocal();
    builder = sdk.WasmSdkBuilder.localTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('gets epochs info and finalized epochs', async function getsEpochsInfo() {
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

  it('queries evonode proposed blocks by id/range', async function queriesEvonodeBlocks() {
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

  it('queries evonode proposed blocks by ids with proof', async () => {
    const EVONODE_ID = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    const res = await client.getEvonodesProposedEpochBlocksByIdsWithProofInfo(8635, [EVONODE_ID]);
    expect(res).to.be.ok();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
  });

  it('queries evonode proposed blocks by range with proof', async () => {
    const res = await client.getEvonodesProposedEpochBlocksByRangeWithProofInfo({
      epoch: 8635,
      limit: 50,
    });
    expect(res).to.be.ok();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
  });
});

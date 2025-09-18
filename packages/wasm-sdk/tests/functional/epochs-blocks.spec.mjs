import init, * as sdk from '../../dist/sdk.js';

describe('Epochs and evonode blocks', function describeEpochs() {
  this.timeout(60000);

  let client;
  let builder;

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
    const current = await client.getCurrentEpoch().catch(() => 1000);
    const start = Math.max(0, (current || 1000) - 5);
    const infos = await client.getEpochsInfo(start, 5, true);
    expect(infos).to.be.an('array');
    const finalized = await client.getFinalizedEpochInfos(start, 5);
    expect(finalized).to.be.an('array');
  });

  it('queries evonode proposed blocks by id/range', async () => {
    const EVONODE_ID = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    await client.getEvonodesProposedEpochBlocksByIds(8635, [EVONODE_ID]);
    await client.getEvonodesProposedEpochBlocksByRange(EVONODE_ID, 50);
  });
});

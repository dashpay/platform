import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Protocol versions', function describeProtocolVersions() {
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

  it('fetches protocol upgrade state', async () => {
    const state = await client.getProtocolVersionUpgradeState();
    expect(state).to.be.ok();
  });

  it('fetches protocol upgrade state with proof', async () => {
    const res = await client.getProtocolVersionUpgradeStateWithProofInfo();
    expect(res).to.be.ok();
    expect(res.data).to.be.ok();
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
  });

  it('lists protocol upgrade vote statuses', async () => {
    const START_PROTX = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    const res = await client.getProtocolVersionUpgradeVoteStatus(START_PROTX, 50);
    expect(res).to.be.instanceOf(Map);
  });

  it('lists protocol upgrade vote statuses with proof', async () => {
    const START_PROTX = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    const res = await client.getProtocolVersionUpgradeVoteStatusWithProofInfo(START_PROTX, 50);
    expect(res).to.be.ok();
    expect(res.data).to.be.instanceOf(Map);
    expect(res.proof).to.be.ok();
    expect(res.metadata).to.be.ok();
  });
});

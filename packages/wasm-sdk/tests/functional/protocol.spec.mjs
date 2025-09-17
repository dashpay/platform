import init, * as sdk from '../../dist/sdk.js';

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

  it('lists protocol upgrade vote statuses', async () => {
    const START_PROTX = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    const res = await client.getProtocolVersionUpgradeVoteStatus(START_PROTX, 50);
    expect(res).to.be.an('array');
  });
});

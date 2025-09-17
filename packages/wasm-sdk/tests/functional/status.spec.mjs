import init, * as sdk from '../../dist/sdk.js';

describe('Status endpoint', function describeBlock() {
  this.timeout(30000);

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

  it('getStatus', async () => {
    const status = await client.getStatus();
    expect(status).to.be.ok();
  });
});

import init, * as sdk from '../../dist/sdk.js';

describe('System info', function describeSystemInfo() {
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

  it('getCurrentQuorumsInfo', async () => {
    const r = await client.getCurrentQuorumsInfo();
    expect(r).to.be.ok();
  });

  it('getTotalCreditsInPlatform', async () => {
    const r = await client.getTotalCreditsInPlatform();
    expect(r).to.be.an('object');
    expect(String(r.totalCreditsInPlatform)).to.match(/^\d+$/);
  });
});

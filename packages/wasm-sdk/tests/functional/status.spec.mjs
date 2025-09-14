import init, * as sdk from '../../dist/sdk.js';

describe('Status endpoint', function () {
  this.timeout(30000);

  let client;
  let builder;

  before(async function () {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(function () {
    if (client) client.free();

  });

  it('getStatus', async function () {
    const status = await client.getStatus();
    expect(status).to.be.ok;
  });
});

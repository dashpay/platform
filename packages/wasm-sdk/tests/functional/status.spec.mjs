import init, * as sdk from '../../dist/sdk.js';

describe('Status endpoint', function () {
  this.timeout(30000);

  let client;
  let builder;

  before(async function () {
    await init();
    await sdk.prefetch_trusted_quorums_testnet();
    builder = sdk.WasmSdkBuilder.new_testnet_trusted();
    client = await builder.build();
  });

  after(function () {
    if (client) client.free();
    if (builder) builder.free();
  });

  it('get_status', async function () {
    const status = await sdk.get_status(client);
    expect(status).to.be.ok;
  });
});

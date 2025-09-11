import init, * as sdk from '../../dist/sdk.js';

describe('System info (quorums, credits)', function () {
  this.timeout(60000);

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

  it('get_current_quorums_info', async () => {
    const r = await sdk.get_current_quorums_info(client);
    expect(r).to.be.ok;
  });

  it('get_total_credits_in_platform', async () => {
    const r = await sdk.get_total_credits_in_platform(client);
    expect(r).to.be.a('number');
  });
});

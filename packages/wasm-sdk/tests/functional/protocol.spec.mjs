import init, * as sdk from '../../dist/sdk.js';

describe('Protocol versions', function () {
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

  });

  it('fetches protocol upgrade state', async () => {
    const state = await sdk.get_protocol_version_upgrade_state(client);
    expect(state).to.be.ok;
  });

  it('lists protocol upgrade vote statuses', async () => {
    const START_PROTX = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    const res = await sdk.get_protocol_version_upgrade_vote_status(client, START_PROTX, 50);
    expect(res).to.be.an('array');
  });
});

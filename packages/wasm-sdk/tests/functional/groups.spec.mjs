import init, * as sdk from '../../dist/sdk.js';

describe('Group queries', function () {
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

  it('fetches identity groups and group members', async () => {
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    // These calls may fail in offline runs; permit network errors
    await sdk.get_identity_groups(client, IDENTITY, 0, null, null, 10);
    await sdk.get_group_members(client, DPNS_CONTRACT, 0, null, null, 10);
  });

  it('fetches groups data contracts', async () => {
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    await sdk.get_groups_data_contracts(client, [DPNS_CONTRACT]);
  });
});

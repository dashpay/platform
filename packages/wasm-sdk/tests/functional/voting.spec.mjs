import init, * as sdk from '../../dist/sdk.js';

describe('Contested resources & voting', function () {
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

  it('lists contested resources and vote state', async () => {
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const PARENT = 'dash';
    const LABEL = 'therealslimshaddy5';
    await sdk.get_contested_resources(
        client,
        'domain',
        DPNS_CONTRACT,
        'parentNameAndLabel',
        'documents',
        null,
        null,
        50,
        null,
        true,
    );
    await sdk.get_contested_resource_vote_state(
        client,
        DPNS_CONTRACT,
        'domain',
        'parentNameAndLabel',
        [PARENT, LABEL],
        'documentTypeName',
        null,
        null,
        50,
        true,
    );
  });
});

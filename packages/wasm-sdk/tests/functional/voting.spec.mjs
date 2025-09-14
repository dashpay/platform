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
  });

  it('lists contested resources and vote state', async () => {
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const PARENT = 'dash';
    const LABEL = 'therealslimshaddy5';
    try {
      await sdk.get_contested_resources(
        client,
        'domain',
        DPNS_CONTRACT,
        'parentNameAndLabel',
        null,
        50,
        null,
        true,
      );
    } catch (e) {
      const msg = `${e?.message || e}`;
      if (!(msg.includes('network') || msg.includes('connection') || msg.includes('Internal error'))) throw e;
    }
    try {
      await sdk.get_contested_resource_vote_state(
        client,
        DPNS_CONTRACT,
        'domain',
        'parentNameAndLabel',
        [PARENT, LABEL],
        'documents',
        null,
        null,
        50,
        true,
      );
    } catch (e) {
      const msg = `${e?.message || e}`;
      if (!(msg.includes('network') || msg.includes('connection') || msg.includes('Internal error'))) throw e;
    }
  });
});

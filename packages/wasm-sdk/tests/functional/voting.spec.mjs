import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Contested resources & voting', function describeContestedResources() {
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
    if (client) {
      client.free();
    }
  });

  it('lists contested resources and vote state', async () => {
    const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const PARENT = 'dash';
    const LABEL = 'therealslimshaddy5';

    await client.getContestedResources(
      'domain',
      DPNS_CONTRACT,
      'parentNameAndLabel',
      null,
      50,
      null,
      true,
    );

    await client.getContestedResourceVoteState(
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
  });
});

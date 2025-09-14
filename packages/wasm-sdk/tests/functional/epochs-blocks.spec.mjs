import init, * as sdk from '../../dist/sdk.js';

describe('Epochs and evonode blocks', function () {
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

  it('gets epochs info and finalized epochs', async () => {
    const current = await sdk.get_current_epoch(client).catch(() => 1000);
    const start = Math.max(0, (current || 1000) - 5);
    const infos = await sdk.get_epochs_info(client, start, 5, true);
    expect(infos).to.be.an('array');
    const finalized = await sdk.get_finalized_epoch_infos(client, start, 5);
    expect(finalized).to.be.an('array');
  });

  it('queries evonode proposed blocks by id/range', async () => {
    const EVONODE_ID = '143dcd6a6b7684fde01e88a10e5d65de9a29244c5ecd586d14a342657025f113';
    await sdk.get_evonodes_proposed_epoch_blocks_by_ids(client, 8635, [EVONODE_ID]);
    await sdk.get_evonodes_proposed_epoch_blocks_by_range(client, EVONODE_ID, 50);
  });
});

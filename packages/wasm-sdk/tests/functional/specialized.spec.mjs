import init, * as sdk from '../../dist/sdk.js';

describe('Specialized queries (masternode, balances)', function () {
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

  it('rejects invalid masternode proTxHash for status/score', async () => {
    await sdk.get_masternode_status(client, '0'.repeat(64)).then(() => { throw new Error('should fail'); }).catch(() => {});
    await sdk.get_masternode_score(client, '0'.repeat(64), 1).then(() => { throw new Error('should fail'); }).catch(() => {});
  });

  it('rejects invalid specialized balance doc id', async () => {
    await sdk.get_prefunded_specialized_balance(client, 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec', 'invalid')
      .then(() => { throw new Error('should fail'); })
      .catch(() => {});
  });
});

import init, * as sdk from '../../dist/sdk.js';

describe('WasmSdkError shape (functional)', function () {
  this.timeout(60000);

  let client;
  let builder;

  const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

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

  it('get_document with bad ID exposes InvalidArgument', async () => {
    try {
      await sdk.get_document(client, DPNS_CONTRACT, 'domain', 'notARealId');
      throw new Error('expected to throw');
    } catch (e) {
      expect(e).to.be.instanceOf(Error);
      expect(e.name).to.equal('InvalidArgument');
      expect(e.message).to.match(/Invalid document ID/i);
      expect(e.retriable).to.equal(false);
    }
  });

  it('get_dpns_usernames with bad identity exposes InvalidArgument', async () => {
    try {
      await sdk.get_dpns_usernames(client, 'badId', 1);
      throw new Error('expected to throw');
    } catch (e) {
      expect(e.name).to.equal('InvalidArgument');
      expect(e.message).to.match(/Invalid identity ID/i);
    }
  });
});


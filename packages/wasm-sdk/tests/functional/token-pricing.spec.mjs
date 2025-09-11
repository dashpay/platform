import init, * as sdk from '../../dist/sdk.js';

describe('Functional: token pricing', function () {
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

  it('calculates token id and fetches price by contract', async () => {
    const CONTRACT_ID = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
    const tokenId = sdk.calculate_token_id_from_contract(CONTRACT_ID, 0);
    expect(tokenId).to.be.a('string');
    try {
      const info = await sdk.get_token_price_by_contract(client, CONTRACT_ID, 0);
      expect(info).to.be.ok;
    } catch (e) {
      const msg = `${e?.message || e}`;
      if (!(msg.includes('No pricing schedule') || msg.includes('Token not found'))) {
        throw e;
      }
    }
  });
});

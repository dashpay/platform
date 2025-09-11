import init, * as sdk from '../../dist/sdk.js';

describe('Functional: token queries (status, prices, info)', function () {
  this.timeout(60000);

  const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
  const TOKEN_CONTRACT_1 = 'Hqyu8WcRwXCTwbNxdga4CN5gsVEGc67wng4TFzceyLUv';
  const TOKEN_CONTRACT_2 = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
  const TOKEN_CONTRACT_3 = 'EETVvWgohFDKtbB3ejEzBcDRMNYkc9TtgXY6y8hzP3Ta';

  let client;
  let builder;

  before(async function () {
    await init();
    builder = sdk.WasmSdkBuilder.new_testnet_trusted();
    client = await builder.build();
  });

  after(function () {
    if (client) client.free();
    if (builder) builder.free();
  });

  it('get_token_total_supply using derived token id', async () => {
    try {
      const tokenId = sdk.calculate_token_id_from_contract(TOKEN_CONTRACT_2, 0);
      const total = await sdk.get_token_total_supply(client, tokenId);
      expect(total).to.be.a('number');
    } catch (e) {
      if (!(`${e?.message || e}`.includes('network') || `${e?.message || e}`.includes('connection'))) throw e;
    }
  });

  it('get_token_statuses for multiple tokens', async () => {
    try {
      await sdk.get_token_statuses(client, [TOKEN_CONTRACT_1, TOKEN_CONTRACT_2]);
    } catch (e) {
      if (!(`${e?.message || e}`.includes('network') || `${e?.message || e}`.includes('connection'))) throw e;
    }
  });

  it('get_token_direct_purchase_prices', async () => {
    try {
      await sdk.get_token_direct_purchase_prices(client, [TOKEN_CONTRACT_2]);
    } catch (e) {
      if (!(`${e?.message || e}`.includes('network') || `${e?.message || e}`.includes('connection'))) throw e;
    }
  });

  it('get_token_contract_info', async () => {
    try {
      await sdk.get_token_contract_info(client, TOKEN_CONTRACT_3);
    } catch (e) {
      if (!(`${e?.message || e}`.includes('network') || `${e?.message || e}`.includes('connection'))) throw e;
    }
  });

  it('get_token_perpetual_distribution_last_claim', async () => {
    try {
      await sdk.get_token_perpetual_distribution_last_claim(client, TEST_IDENTITY, TOKEN_CONTRACT_3);
    } catch (e) {
      if (!(`${e?.message || e}`.includes('network') || `${e?.message || e}`.includes('connection'))) throw e;
    }
  });
});

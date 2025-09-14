import init, * as sdk from '../../dist/sdk.js';

describe('Token queries', function () {
  this.timeout(60000);

  const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
  const TOKEN_CONTRACT = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
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
  });

  // TODO: fix this test

  it.skip('get_token_total_supply using derived token id', async () => {
    const tokenId = sdk.calculate_token_id_from_contract(TOKEN_CONTRACT, 0);
    const total = await sdk.get_token_total_supply(client, tokenId);
    // Returns an object with totalSupply as string
    expect(total).to.be.an('object');
    expect(String(total.totalSupply)).to.match(/^\d+$/);
  });

  it('get_token_statuses for multiple tokens', async () => {
    await sdk.get_token_statuses(client, [TOKEN_CONTRACT]);
  });

  it('get_token_direct_purchase_prices', async () => {
    await sdk.get_token_direct_purchase_prices(client, [TOKEN_CONTRACT_2]);
  });

  it('get_token_contract_info', async () => {
    await sdk.get_token_contract_info(client, TOKEN_CONTRACT_3);
  });

  it('get_token_perpetual_distribution_last_claim', async () => {
    await sdk.get_token_perpetual_distribution_last_claim(client, TEST_IDENTITY, TOKEN_CONTRACT_3);
  });
});

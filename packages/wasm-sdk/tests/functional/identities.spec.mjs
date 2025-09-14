import init, * as sdk from '../../dist/sdk.js';

describe('Identity queries', function () {
  this.timeout(90000);

  const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
  const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

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

  it('fetches identity and basic fields', async () => {
    const r = await sdk.identity_fetch(client, TEST_IDENTITY);
    expect(r).to.be.ok;
  });

  it('gets identity balance and nonce', async () => {
    const bal = await sdk.get_identity_balance(client, TEST_IDENTITY);
    expect(bal).to.be.an('object');
    expect(String(bal.balance)).to.match(/^\d+$/);

    const nonce = await sdk.get_identity_nonce(client, TEST_IDENTITY);
    expect(nonce).to.be.an('object');
    expect(String(nonce.nonce)).to.match(/^\d+$/);
  });

  it('gets contract nonce and keys', async () => {
    await sdk.get_identity_contract_nonce(client, TEST_IDENTITY, DPNS_CONTRACT);
    const keys = await sdk.get_identity_keys(client, TEST_IDENTITY, 'all');
    expect(keys).to.be.an('array');
  });

  it('batch identity balances and balance+revision', async () => {
    const balances = await sdk.get_identities_balances(client, [TEST_IDENTITY]);
    expect(balances).to.be.an('array');
    const br = await sdk.get_identity_balance_and_revision(client, TEST_IDENTITY);
    expect(br).to.be.ok;
  });

  it('contract keys for identity', async () => {
    const r = await sdk.get_identities_contract_keys(client, [TEST_IDENTITY], DPNS_CONTRACT);
    expect(r).to.be.an('array');
  });

  it('token balances/infos for identity and batches', async () => {
    const TOKEN_CONTRACT = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
    const tokenId = sdk.calculate_token_id_from_contract(TOKEN_CONTRACT, 1);

    await sdk.get_identity_token_balances(client, TEST_IDENTITY, [tokenId]);
    await sdk.get_identities_token_balances(client, [TEST_IDENTITY], tokenId);
    await sdk.get_identity_token_infos(client, TEST_IDENTITY, [tokenId]);
    await sdk.get_identities_token_infos(client, [TEST_IDENTITY], 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy');
  });
});

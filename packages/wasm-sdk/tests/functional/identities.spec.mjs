import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Identity queries', function describeBlock() {
  this.timeout(90000);

  const TEST_IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
  const DPNS_CONTRACT = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';

  let client;
  let builder;

  before(async () => {
    await init();
    await sdk.WasmSdk.prefetchTrustedQuorumsTestnet();
    builder = sdk.WasmSdkBuilder.testnetTrusted();
    client = await builder.build();
  });

  after(() => {
    if (client) { client.free(); }
  });

  it('fetches identity and basic fields', async () => {
    const r = await client.getIdentity(TEST_IDENTITY);
    expect(r).to.be.ok();
  });

  it('gets identity balance and nonce', async () => {
    const bal = await client.getIdentityBalance(TEST_IDENTITY);
    expect(bal).to.be.an('object');
    expect(String(bal.balance)).to.match(/^\d+$/);

    const nonce = await client.getIdentityNonce(TEST_IDENTITY);
    expect(nonce).to.be.an('object');
    expect(String(nonce.nonce)).to.match(/^\d+$/);
  });

  it('gets contract nonce and keys', async () => {
    await client.getIdentityContractNonce(TEST_IDENTITY, DPNS_CONTRACT);
    const keys = await client.getIdentityKeys({
      identityId: TEST_IDENTITY,
      request: { type: 'all' },
    });
    expect(keys).to.be.an('array');
  });

  it('batch identity balances and balance+revision', async () => {
    const balances = await client.getIdentitiesBalances([TEST_IDENTITY]);
    expect(balances).to.be.an('array');
    const br = await client.getIdentityBalanceAndRevision(TEST_IDENTITY);
    expect(br).to.be.ok();
  });

  it('contract keys for identity', async () => {
    const r = await client.getIdentitiesContractKeys({
      identityIds: [TEST_IDENTITY],
      contractId: DPNS_CONTRACT,
    });
    expect(r).to.be.an('array');
  });

  it('contract keys filtered by purpose and with proof', async () => {
    const response = await client.getIdentitiesContractKeysWithProofInfo({
      identityIds: [TEST_IDENTITY],
      contractId: DPNS_CONTRACT,
      purposes: [0], // authentication
    });

    // Basic shape
    expect(response).to.be.ok();
    expect(response.metadata).to.be.ok();
    expect(response.proof).to.be.ok();

    // Data sanity: if keys are returned, all should match the requested purpose set
    const keysArray = response.data;
    if (Array.isArray(keysArray) && keysArray.length > 0) {
      for (const entry of keysArray) {
        const keys = entry.keys;
        for (const key of keys) {
          expect(key.purpose).to.match(/Authentication/i);
        }
      }
    }
  });

  it('contract keys batch handles multiple identities', async () => {
    const r = await client.getIdentitiesContractKeys({
      identityIds: [TEST_IDENTITY, TEST_IDENTITY],
      contractId: DPNS_CONTRACT,
    });
    expect(r).to.be.an('array');
    if (r.length > 0) {
      expect(r[0].identityId).to.be.instanceOf(sdk.Identifier); // IdentifierWasm
    }
  });

  it('token balances/infos for identity and batches', async () => {
    const TOKEN_CONTRACT = 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy';
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 1);

    await client.getIdentityTokenBalances(TEST_IDENTITY, [tokenId]);
    await client.getIdentitiesTokenBalances([TEST_IDENTITY], tokenId);
    await client.getIdentityTokenInfos(TEST_IDENTITY, [tokenId]);
    await client.getIdentitiesTokenInfos([TEST_IDENTITY], 'H7FRpZJqZK933r9CzZMsCuf1BM34NT5P2wSJyjDkprqy');
  });
});

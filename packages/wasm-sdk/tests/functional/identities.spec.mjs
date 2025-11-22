import init, * as sdk from '../../dist/sdk.compressed.js';
import { wasmFunctionalTestRequirements } from './fixtures/requiredTestData.mjs';

describe('Identity queries', function describeBlock() {
  this.timeout(90000);

  const {
    identityId: TEST_IDENTITY,
    dpnsContractId: DPNS_CONTRACT,
    tokenContracts,
  } = wasmFunctionalTestRequirements();

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

  it('token balances/infos for identity and batches', async () => {
    const TOKEN_CONTRACT = tokenContracts[0].contractId;
    const tokenId = sdk.WasmSdk.calculateTokenIdFromContract(TOKEN_CONTRACT, 1);

    await client.getIdentityTokenBalances(TEST_IDENTITY, [tokenId]);
    await client.getIdentitiesTokenBalances([TEST_IDENTITY], tokenId);
    await client.getIdentityTokenInfos(TEST_IDENTITY, [tokenId]);
    await client.getIdentitiesTokenInfos([TEST_IDENTITY], tokenContracts[0].contractId);
  });
});

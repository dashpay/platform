import init, * as sdk from '../../dist/sdk.js';

describe('Proofs (verification + fetch with proof)', function () {
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

  it('rejects invalid/malformed proof blobs', async () => {
    await Promise.all([
      sdk.verify_proof(client, '').then(() => { throw new Error('should fail'); }).catch(() => {}),
      sdk.verify_proof(client, 'not-a-valid-proof').then(() => { throw new Error('should fail'); }).catch(() => {}),
      sdk.verify_proofs(client, JSON.stringify(['invalid1','invalid2'])).then(() => { throw new Error('should fail'); }).catch(() => {}),
    ]);
  });

  it('fetches identity/contract (proof verified internally in trusted mode)', async () => {
    const IDENTITY = '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk';
    const DPNS = 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec';
    const identity = await sdk.identity_fetch(client, IDENTITY);
    expect(identity).to.be.ok;
    const contract = await sdk.data_contract_fetch(client, DPNS);
    expect(contract).to.be.ok;
  });
});

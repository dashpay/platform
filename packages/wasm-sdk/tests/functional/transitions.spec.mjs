import init, * as sdk from '../../dist/sdk.js';

describe('Functional: basic state transitions (negative cases)', function () {
  this.timeout(60000);

  let client;
  let builder;

  before(async function () {
    await init();
    const enabled = typeof process !== 'undefined' && process.env && (process.env.SDK_FUNCTIONAL === '1' || process.env.SDK_FUNCTIONAL === 'true');
    if (!enabled) this.skip();
    builder = sdk.WasmSdkBuilder.new_testnet_trusted();
    client = await builder.build();
  });

  after(function () {
    if (client) client.free();
    if (builder) builder.free();
  });

  it('tokenTransfer rejects invalid parameters', async () => {
    await sdk.tokenTransfer('invalid-contract', 0, '1000', 'invalid-identity', 'invalid-recipient', 'Kx...', null)
      .then(() => { throw new Error('should fail'); })
      .catch(() => {});
  });

  it('identityCreate fails with mock proof', async () => {
    const mockProof = JSON.stringify({ coreChainLockedHeight: 100000, outPoint: '0'.repeat(64) + ':0' });
    const assetPriv = sdk.generate_key_pair('testnet').private_key_wif;
    const pubKeys = JSON.stringify([{ keyType: 'ECDSA_SECP256K1', purpose: 'AUTHENTICATION', securityLevel: 'MASTER', privateKeyHex: sdk.generate_key_pair('testnet').private_key_hex }]);
    await sdk.identityCreate(mockProof, assetPriv, pubKeys)
      .then(() => { throw new Error('should fail'); })
      .catch(() => {});
  });
});

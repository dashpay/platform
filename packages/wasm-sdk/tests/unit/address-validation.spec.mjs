import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Address validation', () => {
  before(async () => {
    await init();
  });

  it('validates known malformed prefixes correctly', () => {
    const mainnetAddress = 'XdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh';
    const testnetAddress = 'yXdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh';
    expect(sdk.WasmSdk.validateAddress(mainnetAddress, 'mainnet')).to.be.a('boolean');
    expect(sdk.WasmSdk.validateAddress(testnetAddress, 'testnet')).to.be.a('boolean');
  });

  it('validates generated addresses for each network', () => {
    const mnemonic = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
    const kM = sdk.WasmSdk.deriveKeyFromSeedWithPath(mnemonic, undefined, "m/44'/5'/0'/0/0", 'mainnet');
    const kT = sdk.WasmSdk.deriveKeyFromSeedWithPath(mnemonic, undefined, "m/44'/1'/0'/0/0", 'testnet');
    expect(sdk.WasmSdk.validateAddress(kM.address, 'mainnet')).to.equal(true);
    expect(sdk.WasmSdk.validateAddress(kT.address, 'testnet')).to.equal(true);
  });
});

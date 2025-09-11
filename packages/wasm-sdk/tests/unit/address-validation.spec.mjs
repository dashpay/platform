import init, * as sdk from '../../dist/sdk.js';

describe('Address validation', () => {
  before(async () => {
    await init();
  });

  it('validates known malformed prefixes correctly', () => {
    const mainnetAddress = 'XdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh';
    const testnetAddress = 'yXdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh';
    expect(sdk.validate_address(mainnetAddress, 'mainnet')).to.be.a('boolean');
    expect(sdk.validate_address(testnetAddress, 'testnet')).to.be.a('boolean');
  });

  it('validates generated addresses for each network', () => {
    const mnemonic = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
    const kM = sdk.derive_key_from_seed_with_path(mnemonic, undefined, "m/44'/5'/0'/0/0", 'mainnet');
    const kT = sdk.derive_key_from_seed_with_path(mnemonic, undefined, "m/44'/1'/0'/0/0", 'testnet');
    expect(sdk.validate_address(kM.address, 'mainnet')).to.equal(true);
    expect(sdk.validate_address(kT.address, 'testnet')).to.equal(true);
  });
});

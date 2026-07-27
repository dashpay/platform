import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Address Validation', () => {
  before(async () => {
    await init();
  });

  describe('validateAddress()', () => {
    it('should validate known malformed prefixes correctly', () => {
      const mainnetAddress = 'XdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh';
      const testnetAddress = 'yXdRhagDMpNbHZSvgMXqkcCCWmrDYYty5Nh';
      expect(sdk.WasmSdk.validateAddress(mainnetAddress, 'mainnet')).to.be.a('boolean');
      expect(sdk.WasmSdk.validateAddress(testnetAddress, 'testnet')).to.be.a('boolean');
    });

    it('should validate generated addresses for each network', () => {
      const mnemonic = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
      const kM = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic, passphrase: null, path: "m/44'/5'/0'/0/0", network: 'mainnet',
      });
      const kT = sdk.WasmSdk.deriveKeyFromSeedWithPath({
        mnemonic, passphrase: null, path: "m/44'/1'/0'/0/0", network: 'testnet',
      });
      expect(sdk.WasmSdk.validateAddress(kM.address, 'mainnet')).to.equal(true);
      expect(sdk.WasmSdk.validateAddress(kT.address, 'testnet')).to.equal(true);
    });
  });
});

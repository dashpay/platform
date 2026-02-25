import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('WasmSdkError', () => {
  before(async () => {
    await init();
  });

  describe('generateKeyPair()', () => {
    it('should expose InvalidArgument for invalid network', () => {
      try {
        sdk.WasmSdk.generateKeyPair('invalid_network');
        expect.fail('expected to throw');
      } catch (e) {
        // wasm-bindgen returns our WasmSdkError as an object, not necessarily instanceof Error
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/unsupported network name/i);
        expect(e.retriable).to.not.be.ok();
        expect(e.code).to.equal(-1);
      }
    });
  });

  describe('keyPairFromHex()', () => {
    it('should expose InvalidArgument for invalid hex', () => {
      try {
        sdk.WasmSdk.keyPairFromHex('zzzz', 'mainnet');
        expect.fail('expected to throw');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.retriable).to.not.be.ok();
        // either length or content validation may trigger first
        expect(e.message).to.match(/Invalid hex|must be exactly 64/i);
      }
    });
  });

  describe('deriveKeyFromSeedWithPath()', () => {
    it('should expose InvalidArgument for invalid network', () => {
      const seed = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
      const path = "m/44'/5'/0'/0/0";
      try {
        sdk.WasmSdk.deriveKeyFromSeedWithPath({
          mnemonic: seed, passphrase: null, path, network: 'bogus',
        });
        expect.fail('expected to throw');
      } catch (e) {
        expect(e).to.be.instanceOf(sdk.WasmSdkError);
        expect(e.name).to.equal('InvalidArgument');
        expect(e.message).to.match(/unsupported network name/i);
      }
    });
  });
});

import init, * as sdk from '../../dist/sdk.compressed.js';

describe('WasmSdkError shape (unit)', () => {
  before(async () => {
    await init();
  });

  it('invalid network on generateKeyPair exposes InvalidArgument', () => {
    try {
      sdk.WasmSdk.generateKeyPair('devnet');
      expect.fail('expected to throw');
    } catch (e) {
      // wasm-bindgen returns our WasmSdkError as an object, not necessarily instanceof Error
      expect(e).to.be.instanceOf(sdk.WasmSdkError);
      expect(e.name).to.equal('InvalidArgument');
      expect(e.message).to.match(/Invalid network/i);
      expect(e.retriable).to.equal(false);
      expect(e.code).to.equal(-1);
    }
  });

  it('invalid hex on keyPairFromHex exposes InvalidArgument', () => {
    try {
      sdk.WasmSdk.keyPairFromHex('zzzz', 'mainnet');
      expect.fail('expected to throw');
    } catch (e) {
      expect(e).to.be.instanceOf(sdk.WasmSdkError);
      expect(e.name).to.equal('InvalidArgument');
      expect(e.retriable).to.equal(false);
      // either length or content validation may trigger first
      expect(e.message).to.match(/Invalid hex|must be exactly 64/i);
    }
  });

  it('invalid derivation path network exposes InvalidArgument', () => {
    const seed = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';
    const path = "m/44'/5'/0'/0/0";
    try {
      sdk.WasmSdk.deriveKeyFromSeedWithPath(seed, undefined, path, 'bogus');
      expect.fail('expected to throw');
    } catch (e) {
      expect(e).to.be.instanceOf(sdk.WasmSdkError);
      expect(e.name).to.equal('InvalidArgument');
      expect(e.message).to.match(/Invalid network/i);
    }
  });
});

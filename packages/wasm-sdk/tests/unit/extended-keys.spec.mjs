import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Extended keys', () => {
  const TEST_MNEMONIC = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  before(async () => {
    await init();
  });

  describe('deriveChildPublicKey - basic functionality', () => {
    it('derives non-hardened child xpubs that differ by index', () => {
      const master = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );
      const parentXpub = master.xpub;

      const child0 = sdk.WasmSdk.deriveChildPublicKey(parentXpub, 0, false);
      expect(child0).to.be.a('string');
      expect(child0).to.not.equal(parentXpub);

      const child1 = sdk.WasmSdk.deriveChildPublicKey(parentXpub, 1, false);
      expect(child1).to.be.a('string');
      expect(child1).to.not.equal(child0);
    });
  });

  describe('xprvToXpub - basic functionality', () => {
    it('converts xprv to the expected xpub', () => {
      const master = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );

      const derivedXpub = sdk.WasmSdk.xprvToXpub(master.xprv);
      expect(derivedXpub).to.be.a('string');
      expect(derivedXpub).to.equal(master.xpub);
    });
  });

  describe('deriveChildPublicKey - error handling', () => {
    it('throws when hardened=true', () => {
      const master = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );
      const parentXpub = master.xpub;
      expect(() => sdk.WasmSdk.deriveChildPublicKey(parentXpub, 0, true))
        .to.throw('Cannot derive hardened child from extended public key');
    });

    it('throws when index is in hardened range', () => {
      const master = sdk.WasmSdk.deriveKeyFromSeedWithExtendedPath(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );
      const parentXpub = master.xpub;
      // 0x80000000 == 2^31
      expect(() => sdk.WasmSdk.deriveChildPublicKey(parentXpub, 0x80000000, false))
        .to.throw('Index is in hardened range');
    });

    it('throws for invalid xpub input', () => {
      expect(() => sdk.WasmSdk.deriveChildPublicKey('invalid_xpub', 0, false))
        .to.throw('Invalid extended public key');
    });
  });

  describe('xprvToXpub - error handling', () => {
    it('throws for invalid xprv input', () => {
      expect(() => sdk.WasmSdk.xprvToXpub('invalid_xprv'))
        .to.throw('Invalid extended private key');
    });
  });
});

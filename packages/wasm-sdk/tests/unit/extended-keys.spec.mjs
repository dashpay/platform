import init, * as sdk from '../../dist/sdk.js';

describe('Extended keys', () => {
  const TEST_MNEMONIC = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  before(async () => {
    await init();
  });

  describe('derive_child_public_key - basic functionality', () => {
    it('derives non-hardened child xpubs that differ by index', () => {
      const master = sdk.derive_key_from_seed_with_extended_path(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );
      const parentXpub = master.xpub;

      const child0 = sdk.derive_child_public_key(parentXpub, 0, false);
      expect(child0).to.be.a('string');
      expect(child0).to.not.equal(parentXpub);

      const child1 = sdk.derive_child_public_key(parentXpub, 1, false);
      expect(child1).to.be.a('string');
      expect(child1).to.not.equal(child0);
    });
  });

  describe('xprv_to_xpub - basic functionality', () => {
    it('converts xprv to the expected xpub', () => {
      const master = sdk.derive_key_from_seed_with_extended_path(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );

      const derivedXpub = sdk.xprv_to_xpub(master.xprv);
      expect(derivedXpub).to.be.a('string');
      expect(derivedXpub).to.equal(master.xpub);
    });
  });

  describe('derive_child_public_key - error handling', () => {
    it('throws when hardened=true', () => {
      const master = sdk.derive_key_from_seed_with_extended_path(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );
      const parentXpub = master.xpub;
      expect(() => sdk.derive_child_public_key(parentXpub, 0, true))
        .to.throw('Cannot derive hardened child from extended public key');
    });

    it('throws when index is in hardened range', () => {
      const master = sdk.derive_key_from_seed_with_extended_path(
        TEST_MNEMONIC,
        null,
        "m/44'/5'/0'",
        'mainnet',
      );
      const parentXpub = master.xpub;
      // 0x80000000 == 2^31
      expect(() => sdk.derive_child_public_key(parentXpub, 0x80000000, false))
        .to.throw('Index is in hardened range');
    });

    it('throws for invalid xpub input', () => {
      expect(() => sdk.derive_child_public_key('invalid_xpub', 0, false))
        .to.throw('Invalid extended public key');
    });
  });

  describe('xprv_to_xpub - error handling', () => {
    it('throws for invalid xprv input', () => {
      expect(() => sdk.xprv_to_xpub('invalid_xprv'))
        .to.throw('Invalid extended private key');
    });
  });
});


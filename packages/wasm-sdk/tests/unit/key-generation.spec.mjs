import init, * as sdk from '../../dist/sdk.js';

describe('Keys and mnemonics', () => {
  const TEST_MNEMONIC = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  before(async () => {
    await init();
  });

  describe('mnemonic', () => {
    it('generates 12 and 24 words and validates', () => {
      const m12 = sdk.generate_mnemonic(12);
      expect(m12.split(' ').length).to.equal(12);
      expect(sdk.validate_mnemonic(m12)).to.equal(true);

      const m24 = sdk.generate_mnemonic(24);
      expect(m24.split(' ').length).to.equal(24);
      expect(sdk.validate_mnemonic(m24)).to.equal(true);
    });

    it('supports language wordlists', () => {
      const langs = ['en', 'es', 'fr', 'it', 'ja', 'ko', 'pt', 'cs'];
      for (const lang of langs) {
        const m = sdk.generate_mnemonic(12, lang);
        expect(sdk.validate_mnemonic(m, lang)).to.equal(true);
      }
    });

    it('converts mnemonic to seed (with/without passphrase)', () => {
      const seed = sdk.mnemonic_to_seed(TEST_MNEMONIC);
      expect(seed && seed.length).to.equal(64);

      const seed2 = sdk.mnemonic_to_seed(TEST_MNEMONIC, 'passphrase');
      expect(seed2 && seed2.length).to.equal(64);
      expect(Buffer.from(seed2).toString('hex')).to.not.equal(Buffer.from(seed).toString('hex'));
    });
  });

  describe('key pairs and addresses', () => {
    it('generates key pairs for mainnet/testnet', () => {
      const kpM = sdk.generate_key_pair('mainnet');
      expect(kpM.address.startsWith('X')).to.equal(true);
      const kpT = sdk.generate_key_pair('testnet');
      expect(kpT.address.startsWith('y')).to.equal(true);
    });

    it('derives address from pubkey equals generated address', () => {
      const kp = sdk.generate_key_pair('mainnet');
      const addr = sdk.pubkey_to_address(kp.public_key, 'mainnet');
      expect(addr).to.equal(kp.address);
    });

    it('signs messages deterministically for same inputs', () => {
      const kp = sdk.generate_key_pair('mainnet');
      const msg = 'Hello, Dash!';
      const s1 = sdk.sign_message(msg, kp.private_key_wif);
      const s2 = sdk.sign_message(msg, kp.private_key_wif);
      expect(s1).to.be.a('string');
      expect(s1).to.equal(s2);
    });
  });
});

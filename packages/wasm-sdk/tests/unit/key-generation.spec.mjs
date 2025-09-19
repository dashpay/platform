import init, * as sdk from '../../dist/sdk.js';

describe('Keys and mnemonics', () => {
  const TEST_MNEMONIC = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  before(async () => {
    await init();
  });

  describe('mnemonic', () => {
    it('generates 12 and 24 words and validates', () => {
      const m12 = sdk.WasmSdk.generateMnemonic(12);
      expect(m12.split(' ').length).to.equal(12);
      expect(sdk.WasmSdk.validateMnemonic(m12)).to.equal(true);

      const m24 = sdk.WasmSdk.generateMnemonic(24);
      expect(m24.split(' ').length).to.equal(24);
      expect(sdk.WasmSdk.validateMnemonic(m24)).to.equal(true);
    });

    it('supports language wordlists', () => {
      const langs = ['en', 'es', 'fr', 'it', 'ja', 'ko', 'pt', 'cs'];
      for (const lang of langs) {
        const m = sdk.WasmSdk.generateMnemonic(12, lang);
        expect(sdk.WasmSdk.validateMnemonic(m, lang)).to.equal(true);
      }
    });

    it('converts mnemonic to seed (with/without passphrase)', () => {
      const seed = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC);
      expect(seed && seed.length).to.equal(64);

      const seed2 = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC, 'passphrase');
      expect(seed2 && seed2.length).to.equal(64);
      expect(Buffer.from(seed2).toString('hex')).to.not.equal(Buffer.from(seed).toString('hex'));
    });
  });

  describe('key pairs and addresses', () => {
    it('generates key pairs for mainnet/testnet', () => {
      const kpM = sdk.WasmSdk.generateKeyPair('mainnet');
      expect(kpM.address.startsWith('X')).to.equal(true);
      const kpT = sdk.WasmSdk.generateKeyPair('testnet');
      expect(kpT.address.startsWith('y')).to.equal(true);
    });

    it('derives address from pubkey equals generated address', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const addr = sdk.WasmSdk.pubkeyToAddress(kp.public_key, 'mainnet');
      expect(addr).to.equal(kp.address);
    });

    it('signs messages deterministically for same inputs', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const msg = 'Hello, Dash!';
      const s1 = sdk.WasmSdk.signMessage(msg, kp.private_key_wif);
      const s2 = sdk.WasmSdk.signMessage(msg, kp.private_key_wif);
      expect(s1).to.be.a('string');
      expect(s1).to.equal(s2);
    });
  });
});

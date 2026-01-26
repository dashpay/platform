import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('Keys and mnemonics', () => {
  const TEST_MNEMONIC = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  before(async () => {
    await init();
  });

  describe('mnemonic', () => {
    it('should generate 12 and 24 words and validate', () => {
      const m12 = sdk.WasmSdk.generateMnemonic({ wordCount: 12 });
      expect(m12.split(' ').length).to.equal(12);
      expect(sdk.WasmSdk.validateMnemonic(m12)).to.equal(true);

      const m24 = sdk.WasmSdk.generateMnemonic({ wordCount: 24 });
      expect(m24.split(' ').length).to.equal(24);
      expect(sdk.WasmSdk.validateMnemonic(m24)).to.equal(true);
    });

    it('should support language wordlists', () => {
      const langs = ['en', 'es', 'fr', 'it', 'ja', 'ko', 'pt', 'cs'];
      for (const lang of langs) {
        const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: lang });
        expect(sdk.WasmSdk.validateMnemonic(m, lang)).to.equal(true);
      }
    });

    it('should convert mnemonic to seed (with/without passphrase)', () => {
      const seed = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC);
      expect(seed && seed.length).to.equal(64);

      const seed2 = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC, 'passphrase');
      expect(seed2 && seed2.length).to.equal(64);
      expect(Buffer.from(seed2).toString('hex')).to.not.equal(Buffer.from(seed).toString('hex'));
    });
  });

  describe('key pairs and addresses', () => {
    it('should generate key pairs for mainnet/testnet', () => {
      const kpM = sdk.WasmSdk.generateKeyPair('mainnet');
      expect(kpM.address.startsWith('X')).to.equal(true);
      const kpT = sdk.WasmSdk.generateKeyPair('testnet');
      expect(kpT.address.startsWith('y')).to.equal(true);
    });

    it('should derive address from pubkey equal to generated address', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const addr = sdk.WasmSdk.pubkeyToAddress(kp.publicKey, 'mainnet');
      expect(addr).to.equal(kp.address);
    });

    it('should sign messages deterministically for same inputs', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const msg = 'Hello, Dash!';
      const s1 = sdk.WasmSdk.signMessage(msg, kp.privateKeyWif);
      const s2 = sdk.WasmSdk.signMessage(msg, kp.privateKeyWif);
      expect(s1).to.be.a('string');
      expect(s1).to.equal(s2);
    });
  });
});

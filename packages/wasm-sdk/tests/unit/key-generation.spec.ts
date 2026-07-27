import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

describe('WasmSdk', () => {
  const TEST_MNEMONIC = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about';

  before(async () => {
    await init();
  });

  describe('generateMnemonic()', () => {
    it('should generate a 12-word mnemonic', () => {
      const m12 = sdk.WasmSdk.generateMnemonic({ wordCount: 12 });
      expect(m12.split(' ').length).to.equal(12);
    });

    it('should generate a 24-word mnemonic', () => {
      const m24 = sdk.WasmSdk.generateMnemonic({ wordCount: 24 });
      expect(m24.split(' ').length).to.equal(24);
    });

    it('should generate valid 12-word mnemonic', () => {
      const m12 = sdk.WasmSdk.generateMnemonic({ wordCount: 12 });
      expect(sdk.WasmSdk.validateMnemonic(m12)).to.equal(true);
    });

    it('should generate valid 24-word mnemonic', () => {
      const m24 = sdk.WasmSdk.generateMnemonic({ wordCount: 24 });
      expect(sdk.WasmSdk.validateMnemonic(m24)).to.equal(true);
    });

    it('should support English wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'en' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'en')).to.equal(true);
    });

    it('should support Spanish wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'es' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'es')).to.equal(true);
    });

    it('should support French wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'fr' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'fr')).to.equal(true);
    });

    it('should support Italian wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'it' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'it')).to.equal(true);
    });

    it('should support Japanese wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'ja' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'ja')).to.equal(true);
    });

    it('should support Korean wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'ko' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'ko')).to.equal(true);
    });

    it('should support Portuguese wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'pt' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'pt')).to.equal(true);
    });

    it('should support Czech wordlist', () => {
      const m = sdk.WasmSdk.generateMnemonic({ wordCount: 12, languageCode: 'cs' });
      expect(sdk.WasmSdk.validateMnemonic(m, 'cs')).to.equal(true);
    });
  });

  describe('validateMnemonic()', () => {
    it('should validate a correct mnemonic', () => {
      expect(sdk.WasmSdk.validateMnemonic(TEST_MNEMONIC)).to.equal(true);
    });
  });

  describe('mnemonicToSeed()', () => {
    it('should convert mnemonic to 64-byte seed', () => {
      const seed = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC);
      expect(seed && seed.length).to.equal(64);
    });

    it('should convert mnemonic to seed with passphrase', () => {
      const seed = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC, 'passphrase');
      expect(seed && seed.length).to.equal(64);
    });

    it('should produce different seeds with and without passphrase', () => {
      const seed = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC);
      const seed2 = sdk.WasmSdk.mnemonicToSeed(TEST_MNEMONIC, 'passphrase');
      expect(Buffer.from(seed2).toString('hex')).to.not.equal(Buffer.from(seed).toString('hex'));
    });
  });

  describe('generateKeyPair()', () => {
    it('should generate mainnet address starting with X', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      expect(kp.address.startsWith('X')).to.equal(true);
    });

    it('should generate testnet address starting with y', () => {
      const kp = sdk.WasmSdk.generateKeyPair('testnet');
      expect(kp.address.startsWith('y')).to.equal(true);
    });
  });

  describe('pubkeyToAddress()', () => {
    it('should derive address from pubkey equal to generated address', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const addr = sdk.WasmSdk.pubkeyToAddress(kp.publicKey, 'mainnet');
      expect(addr).to.equal(kp.address);
    });
  });

  describe('signMessage()', () => {
    it('should return a string signature', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const msg = 'Hello, Dash!';
      const sig = sdk.WasmSdk.signMessage(msg, kp.privateKeyWif);
      expect(sig).to.be.a('string');
    });

    it('should produce deterministic signatures for same inputs', () => {
      const kp = sdk.WasmSdk.generateKeyPair('mainnet');
      const msg = 'Hello, Dash!';
      const s1 = sdk.WasmSdk.signMessage(msg, kp.privateKeyWif);
      const s2 = sdk.WasmSdk.signMessage(msg, kp.privateKeyWif);
      expect(s1).to.equal(s2);
    });
  });
});

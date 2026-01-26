import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('IdentitySigner', () => {
  // Test private key (WIF format for testnet)
  const testPrivateKeyWif = 'cR4EZ2nAvCmn2cFepKn7UgSSQFgFTjkySAchvcoiEVdm48eWjQGn';
  // Same key in hex format (32 bytes)
  const testPrivateKeyHex = '67ad1669d882da256b6fa05e1b0ae384a6ac8aed146ea53602b8ff0e1e9c18e9';

  describe('construction', () => {
    it('should create empty signer', () => {
      const signer = new wasm.IdentitySigner();
      expect(signer).to.exist;
      expect(signer.keyCount).to.equal(0);
    });
  });

  describe('addKey', () => {
    it('should add key from PrivateKey created from WIF', () => {
      const signer = new wasm.IdentitySigner();
      const privateKey = wasm.PrivateKey.fromWIF(testPrivateKeyWif);

      signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);
    });

    it('should add key from PrivateKey created from hex', () => {
      const signer = new wasm.IdentitySigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);
    });

    it('should add key from PrivateKey created from bytes', () => {
      const signer = new wasm.IdentitySigner();
      const keyBytes = new Uint8Array(32).fill(1);
      const privateKey = wasm.PrivateKey.fromBytes(keyBytes, 'testnet');

      signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);
    });

    it('should add multiple keys', () => {
      const signer = new wasm.IdentitySigner();

      const privateKey1 = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      const privateKey2 = wasm.PrivateKey.fromHex(
        'a9d9d0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfd',
        'testnet',
      );

      signer.addKey(privateKey1);
      signer.addKey(privateKey2);

      expect(signer.keyCount).to.equal(2);
    });

    it('should replace key for same public key hash', () => {
      const signer = new wasm.IdentitySigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);

      // Add same key again
      signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1); // Still 1, replaced
    });
  });

  describe('addKeyFromWif', () => {
    it('should add key from WIF string', () => {
      const signer = new wasm.IdentitySigner();

      signer.addKeyFromWif(testPrivateKeyWif);
      expect(signer.keyCount).to.equal(1);
    });

    it('should throw for invalid WIF', () => {
      const signer = new wasm.IdentitySigner();

      expect(() => {
        signer.addKeyFromWif('invalidWif');
      }).to.throw();
    });
  });
});

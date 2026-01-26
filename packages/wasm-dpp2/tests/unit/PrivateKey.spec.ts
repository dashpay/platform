import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';
import { wif, bytes, publicKeyHash } from './mocks/PrivateKey/index.js';
import { fromHexString, toHexString } from './utils/hex.js';

before(async () => {
  await initWasm();
});

describe('PrivateKey', () => {
  describe('fromWIF()', () => {
    it('should create PrivateKey from WIF', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey).to.be.an.instanceof(wasm.PrivateKey);
    });
  });

  describe('fromBytes()', () => {
    it('should create PrivateKey from bytes', () => {
      const pkey = wasm.PrivateKey.fromBytes(fromHexString(bytes), 'Mainnet');

      expect(pkey).to.be.an.instanceof(wasm.PrivateKey);
    });
  });

  describe('fromHex()', () => {
    it('should create PrivateKey from hex', () => {
      const pkey = wasm.PrivateKey.fromBytes(fromHexString(bytes), 'Mainnet');

      const pkeyFromHex = wasm.PrivateKey.fromHex(bytes, 'Mainnet');

      expect(pkey.toBytes()).to.deep.equal(pkeyFromHex.toBytes());
    });
  });

  describe('toWIF()', () => {
    it('should return key as WIF', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.toWIF()).to.equal(wif);
    });
  });

  describe('toBytes()', () => {
    it('should return key as bytes', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.toBytes()).to.deep.equal(fromHexString(bytes));
    });

    it('should return key bytes matching hex string', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(toHexString(pkey.toBytes())).to.equal(bytes);
    });
  });

  describe('toHex()', () => {
    it('should return key as hex', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.toHex().toLowerCase()).to.equal(bytes);
    });
  });

  describe('getPublicKeyHash()', () => {
    it('should return public key hash', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.getPublicKeyHash()).to.deep.equal(publicKeyHash);
    });
  });
});

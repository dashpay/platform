import getWasm from './helpers/wasm.js';
import { wif, bytes, publicKeyHash } from './mocks/PrivateKey/index.js';
import { fromHexString, toHexString } from './utils/hex.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('PrivateKey', () => {
  describe('serialization / deserialization', () => {
    it('should allows to create PrivateKey from wif', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.__wbg_ptr).to.not.equal(0);
    });

    it('should allows to create PrivateKey from bytes', () => {
      const pkey = wasm.PrivateKey.fromBytes(fromHexString(bytes), 'Mainnet');

      expect(pkey.__wbg_ptr).to.not.equal(0);
    });

    it('should allows to create PrivateKey from hex', () => {
      const pkey = wasm.PrivateKey.fromBytes(fromHexString(bytes), 'Mainnet');

      const pkeyFromHex = wasm.PrivateKey.fromHex(bytes, 'Mainnet');

      expect(pkey.bytes()).to.deep.equal(pkeyFromHex.bytes());
    });

    it('should allow to create PrivateKey from wif and read value in wif', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.WIF()).to.equal(wif);
    });

    it('should allow to create PrivateKey from wif and write value in bytes', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.bytes()).to.deep.equal(fromHexString(bytes));
    });
  });

  describe('getters', () => {
    it('should allow to get key wif', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.WIF()).to.equal(wif);
    });

    it('should allow to get key bytes', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(toHexString(pkey.bytes())).to.equal(bytes);
    });

    it('should allow to get key hex', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.toHex().toLowerCase()).to.equal(bytes);
    });

    it('should allow to get public key hash', () => {
      const pkey = wasm.PrivateKey.fromWIF(wif);

      expect(pkey.getPublicKeyHash()).to.deep.equal(publicKeyHash);
    });
  });
});

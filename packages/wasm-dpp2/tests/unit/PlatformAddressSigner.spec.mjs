import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('PlatformAddressSigner', () => {
  // Test private key (WIF format for testnet)
  const testPrivateKeyWif = 'cR4EZ2nAvCmn2cFepKn7UgSSQFgFTjkySAchvcoiEVdm48eWjQGn';
  // Same key in hex format (32 bytes)
  const testPrivateKeyHex = '67ad1669d882da256b6fa05e1b0ae384a6ac8aed146ea53602b8ff0e1e9c18e9';

  describe('construction', () => {
    it('should create empty signer', () => {
      const signer = new wasm.PlatformAddressSigner();
      expect(signer).to.exist;
      expect(signer.keyCount).to.equal(0);
    });
  });

  describe('addKey', () => {
    it('should add key from PrivateKey created from WIF', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const privateKey = wasm.PrivateKey.fromWIF(testPrivateKeyWif);

      signer.addKey(addr, privateKey);
      expect(signer.keyCount).to.equal(1);
    });

    it('should add key from PrivateKey created from hex', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(addr, privateKey);
      expect(signer.keyCount).to.equal(1);
    });

    it('should add key from PrivateKey created from bytes', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const keyBytes = new Uint8Array(32).fill(1);
      const privateKey = wasm.PrivateKey.fromBytes(keyBytes, 'testnet');

      signer.addKey(addr, privateKey);
      expect(signer.keyCount).to.equal(1);
    });

    it('should add multiple keys', () => {
      const signer = new wasm.PlatformAddressSigner();

      const addr1Bytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr1 = wasm.PlatformAddress.fromBytes(addr1Bytes);

      const addr2Bytes = new Uint8Array([0x00, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
      const addr2 = wasm.PlatformAddress.fromBytes(addr2Bytes);

      const privateKey1 = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      const privateKey2 = wasm.PrivateKey.fromHex('a9d9d0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfd', 'testnet');

      signer.addKey(addr1, privateKey1);
      signer.addKey(addr2, privateKey2);

      expect(signer.keyCount).to.equal(2);
    });

    it('should accept address as bech32m string', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const bech32m = addr.toBech32m('testnet');

      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      signer.addKey(bech32m, privateKey);

      expect(signer.keyCount).to.equal(1);
    });

    it('should replace key for same address', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);

      const privateKey1 = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      const privateKey2 = wasm.PrivateKey.fromHex('a9d9d0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfd', 'testnet');

      signer.addKey(addr, privateKey1);
      expect(signer.keyCount).to.equal(1);

      // Add different key for same address
      signer.addKey(addr, privateKey2);
      expect(signer.keyCount).to.equal(1); // Still 1, replaced
    });
  });

  describe('hasKey', () => {
    it('should return true for added address', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(addr, privateKey);
      expect(signer.hasKey(addr)).to.be.true;
    });

    it('should return false for unknown address', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);

      expect(signer.hasKey(addr)).to.be.false;
    });

    it('should accept address as bech32m string', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const bech32m = addr.toBech32m('testnet');
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(addr, privateKey);
      expect(signer.hasKey(bech32m)).to.be.true;
    });
  });

  describe('getPrivateKeysBytes', () => {
    it('should return empty array for empty signer', () => {
      const signer = new wasm.PlatformAddressSigner();
      const keys = signer.getPrivateKeysBytes();
      expect(keys).to.be.an('array');
      expect(keys.length).to.equal(0);
    });

    it('should return array of key entries', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(addr, privateKey);

      const keys = signer.getPrivateKeysBytes();
      expect(keys).to.be.an('array');
      expect(keys.length).to.equal(1);
      expect(keys[0].addressHash).to.be.instanceOf(Uint8Array);
      expect(keys[0].privateKey).to.be.instanceOf(Uint8Array);
      expect(keys[0].addressHash.length).to.equal(20);
      expect(keys[0].privateKey.length).to.equal(32);
    });
  });
});

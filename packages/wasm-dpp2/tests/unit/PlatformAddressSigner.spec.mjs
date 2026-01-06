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
    it('should add key and return derived address from PrivateKey created from WIF', () => {
      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromWIF(testPrivateKeyWif);

      const derivedAddr = signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);
      expect(derivedAddr).to.exist;
      expect(derivedAddr.addressType).to.equal('P2PKH');
    });

    it('should add key and return derived address from PrivateKey created from hex', () => {
      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      const derivedAddr = signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);
      expect(derivedAddr).to.exist;
      expect(derivedAddr.addressType).to.equal('P2PKH');
    });

    it('should add key and return derived address from PrivateKey created from bytes', () => {
      const signer = new wasm.PlatformAddressSigner();
      const keyBytes = new Uint8Array(32).fill(1);
      const privateKey = wasm.PrivateKey.fromBytes(keyBytes, 'testnet');

      const derivedAddr = signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);
      expect(derivedAddr).to.exist;
      expect(derivedAddr.addressType).to.equal('P2PKH');
    });

    it('should add multiple keys with different derived addresses', () => {
      const signer = new wasm.PlatformAddressSigner();

      const privateKey1 = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');
      const privateKey2 = wasm.PrivateKey.fromHex('a9d9d0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfd', 'testnet');

      const addr1 = signer.addKey(privateKey1);
      const addr2 = signer.addKey(privateKey2);

      expect(signer.keyCount).to.equal(2);
      // Addresses should be different since keys are different
      expect(addr1.toBytes()).to.not.deep.equal(addr2.toBytes());
    });

    it('should return same address when adding same key twice', () => {
      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      const addr1 = signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1);

      // Add same key again
      const addr2 = signer.addKey(privateKey);
      expect(signer.keyCount).to.equal(1); // Still 1, same address
      expect(addr1.toBytes()).to.deep.equal(addr2.toBytes());
    });
  });

  describe('hasKey', () => {
    it('should return true for derived address', () => {
      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      const derivedAddr = signer.addKey(privateKey);
      expect(signer.hasKey(derivedAddr)).to.be.true;
    });

    it('should return false for unknown address', () => {
      const signer = new wasm.PlatformAddressSigner();
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromBytes(addressBytes);

      expect(signer.hasKey(addr)).to.be.false;
    });

    it('should accept derived address as bech32m string', () => {
      const signer = new wasm.PlatformAddressSigner();
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      const derivedAddr = signer.addKey(privateKey);
      const bech32m = derivedAddr.toBech32m('testnet');
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
      const privateKey = wasm.PrivateKey.fromHex(testPrivateKeyHex, 'testnet');

      signer.addKey(privateKey);

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

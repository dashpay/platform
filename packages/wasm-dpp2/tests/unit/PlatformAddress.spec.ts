import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';


before(async () => {
  await initWasm();
});

describe('PlatformAddress', () => {
  describe('fromBytes', () => {
    it('should create P2PKH address from bytes', () => {
      const p2pkhBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(p2pkhBytes);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
      expect(addr.isP2pkh).to.be.true();
      expect(addr.isP2sh).to.be.false();
    });

    it('should create P2SH address from bytes', () => {
      const p2shBytes = new Uint8Array([
        0x80, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(p2shBytes);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2SH');
      expect(addr.isP2pkh).to.be.false();
      expect(addr.isP2sh).to.be.true();
    });

    it('should reject invalid address type', () => {
      const invalidBytes = new Uint8Array([
        0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      try {
        wasm.PlatformAddress.fromBytes(invalidBytes);
        expect.fail('Should have thrown error for invalid address type');
      } catch (error: unknown) {
        expect((error as Error).message).to.include('type');
      }
    });

    it('should reject wrong length', () => {
      const shortBytes = new Uint8Array([0xb0, 1, 2, 3, 4, 5]);
      try {
        wasm.PlatformAddress.fromBytes(shortBytes);
        expect.fail('Should have thrown error for wrong length');
      } catch (error: unknown) {
        expect((error as Error).message).to.include('21');
      }
    });
  });

  describe('fromBech32m', () => {
    it('should create address from testnet bech32m string', () => {
      // First create an address and get its bech32m
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const originalAddr = wasm.PlatformAddress.fromBytes(bytes);
      const bech32m = originalAddr.toBech32m('testnet');

      expect(bech32m.startsWith('tevo1')).to.be.true();

      // Parse it back
      const parsedAddr = wasm.PlatformAddress.fromBech32m(bech32m);
      expect(parsedAddr).to.exist;
      expect(parsedAddr.addressType).to.equal('P2PKH');
    });

    it('should create address from mainnet bech32m string', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const originalAddr = wasm.PlatformAddress.fromBytes(bytes);
      const bech32m = originalAddr.toBech32m('mainnet');

      expect(bech32m.startsWith('evo1')).to.be.true();

      const parsedAddr = wasm.PlatformAddress.fromBech32m(bech32m);
      expect(parsedAddr).to.exist;
      expect(parsedAddr.addressType).to.equal('P2PKH');
    });

    it('should reject invalid bech32m string', () => {
      try {
        wasm.PlatformAddress.fromBech32m('invalid-address');
        expect.fail('Should have thrown error for invalid bech32m');
      } catch (error: unknown) {
        expect(error).to.exist;
      }
    });
  });

  describe('fromHex', () => {
    it('should create address from hex string', () => {
      const hexString = `b0${'01020304050607080910111213141516171819'.padEnd(40, '0').substring(0, 40)}`;
      const addr = wasm.PlatformAddress.fromHex(hexString);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
    });

    it('should reject invalid hex', () => {
      try {
        wasm.PlatformAddress.fromHex('not-valid-hex');
        expect.fail('Should have thrown error for invalid hex');
      } catch (error: unknown) {
        expect((error as Error).message).to.include('hex');
      }
    });
  });

  describe('fromP2pkhHash', () => {
    it('should create P2PKH address from 20-byte hash', () => {
      const hash = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromP2pkhHash(hash);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
      expect(addr.isP2pkh).to.be.true();
    });

    it('should reject wrong hash length', () => {
      const shortHash = new Uint8Array([1, 2, 3, 4, 5]);
      try {
        wasm.PlatformAddress.fromP2pkhHash(shortHash);
        expect.fail('Should have thrown error for wrong length');
      } catch (error: unknown) {
        expect((error as Error).message).to.include('20 bytes');
      }
    });
  });

  describe('fromP2shHash', () => {
    it('should create P2SH address from 20-byte hash', () => {
      const hash = new Uint8Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const addr = wasm.PlatformAddress.fromP2shHash(hash);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2SH');
      expect(addr.isP2sh).to.be.true();
    });

    it('should reject wrong hash length', () => {
      const shortHash = new Uint8Array([1, 2, 3, 4, 5]);
      try {
        wasm.PlatformAddress.fromP2shHash(shortHash);
        expect.fail('Should have thrown error for wrong length');
      } catch (error: unknown) {
        expect((error as Error).message).to.include('20 bytes');
      }
    });
  });

  describe('constructor', () => {
    it('should accept PlatformAddress object', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const original = wasm.PlatformAddress.fromBytes(bytes);
      const copy = new wasm.PlatformAddress(original);
      expect(copy).to.exist;
      expect(copy.addressType).to.equal(original.addressType);
    });

    it('should accept Uint8Array', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = new wasm.PlatformAddress(bytes);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
    });

    it('should accept bech32m string', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const original = wasm.PlatformAddress.fromBytes(bytes);
      const bech32m = original.toBech32m('testnet');

      const addr = new wasm.PlatformAddress(bech32m);
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
    });
  });

  describe('toBytes', () => {
    it('should return 21-byte array', () => {
      const inputBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(inputBytes);
      const outputBytes = addr.toBytes();
      expect(outputBytes.length).to.equal(21);
      expect(Array.from(outputBytes)).to.deep.equal(Array.from(inputBytes));
    });
  });

  describe('toHex', () => {
    it('should return hex string', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(bytes);
      const hex = addr.toHex();
      expect(hex).to.be.a('string');
      expect(hex.length).to.equal(42); // 21 bytes * 2
    });
  });

  describe('hash', () => {
    it('should return 20-byte hash', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(bytes);
      const hash = addr.hash();
      expect(hash.length).to.equal(20);
      expect(Array.from(hash)).to.deep.equal([
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
    });
  });

  describe('hashToHex', () => {
    it('should return hex string of hash', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(bytes);
      const hashHex = addr.hashToHex();
      expect(hashHex).to.be.a('string');
      expect(hashHex.length).to.equal(40); // 20 bytes * 2
    });
  });

  describe('toBech32m', () => {
    it('should convert to testnet bech32m', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(bytes);
      const bech32m = addr.toBech32m('testnet');
      expect(bech32m).to.be.a('string');
      expect(bech32m.startsWith('tevo1')).to.be.true();
    });

    it('should convert to mainnet bech32m', () => {
      const bytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(bytes);
      const bech32m = addr.toBech32m('mainnet');
      expect(bech32m).to.be.a('string');
      expect(bech32m.startsWith('evo1')).to.be.true();
    });

    it('should roundtrip through bech32m', () => {
      const originalBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const addr = wasm.PlatformAddress.fromBytes(originalBytes);
      const bech32m = addr.toBech32m('testnet');

      const recovered = wasm.PlatformAddress.fromBech32m(bech32m);
      const recoveredBytes = recovered.toBytes();
      expect(Array.from(recoveredBytes)).to.deep.equal(Array.from(originalBytes));
    });
  });
});

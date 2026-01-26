import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';

before(async () => {
  await initWasm();
});

describe('PlatformAddressOutput', () => {
  describe('construction', () => {
    it('should create from PlatformAddress object', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(500000));
      expect(output).to.exist;
      expect(output.amount).to.equal(BigInt(500000));
    });

    it('should create from bech32m address string', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);
      const bech32m = platformAddr.toBech32m('testnet');

      const output = new wasm.PlatformAddressOutput(bech32m, BigInt(90000));
      expect(output).to.exist;
      expect(output.amount).to.equal(BigInt(90000));
    });

    it('should create from Uint8Array', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);

      const output = new wasm.PlatformAddressOutput(addressBytes, BigInt(100000));
      expect(output).to.exist;
      expect(output.amount).to.equal(BigInt(100000));
    });

    it('should handle large amounts', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const largeAmount = BigInt('10000000000000000');
      const output = new wasm.PlatformAddressOutput(platformAddr, largeAmount);
      expect(output.amount).to.equal(largeAmount);
    });

    it('should handle zero amount', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(0));
      expect(output.amount).to.equal(BigInt(0));
    });

    it('should reject negative amount', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      expect(() => {
        // eslint-disable-next-line no-new
        new wasm.PlatformAddressOutput(platformAddr, BigInt(-1));
      }).to.throw();
    });
  });

  describe('getters', () => {
    it('should return the address', () => {
      // 0x80 is P2SH address type
      const addressBytes = new Uint8Array([
        0x80, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(100000));
      const addr = output.address;
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2SH');
    });

    it('should return the amount', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(999999));
      expect(output.amount).to.equal(BigInt(999999));
    });
  });
});

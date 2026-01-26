import { expect } from './helpers/chai.ts';
import { initWasm, wasm } from '../../dist/dpp.compressed.js';


before(async () => {
  await initWasm();
});

describe('PlatformAddressInput', () => {
  describe('construction', () => {
    it('should create from PlatformAddress object', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, 5, BigInt(500000));
      expect(input).to.exist;
      expect(input.nonce).to.equal(5);
      expect(input.amount).to.equal(BigInt(500000));
    });

    it('should create from bech32m address string', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);
      const bech32m = platformAddr.toBech32m('testnet');

      const input = new wasm.PlatformAddressInput(bech32m, 1, BigInt(100000));
      expect(input).to.exist;
      expect(input.nonce).to.equal(1);
      expect(input.amount).to.equal(BigInt(100000));
    });

    it('should create from Uint8Array', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);

      const input = new wasm.PlatformAddressInput(addressBytes, 0, BigInt(100000));
      expect(input).to.exist;
      expect(input.nonce).to.equal(0);
      expect(input.amount).to.equal(BigInt(100000));
    });

    it('should handle zero nonce', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, 0, BigInt(100000));
      expect(input.nonce).to.equal(0);
    });

    it('should handle large amounts', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const largeAmount = BigInt('10000000000000000'); // 10 quadrillion
      const input = new wasm.PlatformAddressInput(platformAddr, 1, largeAmount);
      expect(input.amount).to.equal(largeAmount);
    });

    it('should handle max u32 nonce', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const maxU32 = 4294967295; // u32::MAX
      const input = new wasm.PlatformAddressInput(platformAddr, maxU32, BigInt(100000));
      expect(input.nonce).to.equal(maxU32);
    });
  });

  describe('getters', () => {
    it('should return the address', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, 1, BigInt(100000));
      const addr = input.address;
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
    });

    it('should return the nonce', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, 42, BigInt(100000));
      expect(input.nonce).to.equal(42);
    });

    it('should return the amount', () => {
      // 0xb0 is P2PKH address type
      const addressBytes = new Uint8Array([
        0xb0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
      ]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, 1, BigInt(123456));
      expect(input.amount).to.equal(BigInt(123456));
    });
  });
});

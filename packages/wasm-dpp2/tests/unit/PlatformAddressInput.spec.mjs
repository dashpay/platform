import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('PlatformAddressInput', () => {
  describe('construction', () => {
    it('should create from PlatformAddress object', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, BigInt(5), BigInt(500000));
      expect(input).to.exist;
      expect(input.nonce).to.equal(BigInt(5));
      expect(input.amount).to.equal(BigInt(500000));
    });

    it('should create from bech32m address string', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);
      const bech32m = platformAddr.toBech32m('testnet');

      const input = new wasm.PlatformAddressInput(bech32m, BigInt(1), BigInt(100000));
      expect(input).to.exist;
      expect(input.nonce).to.equal(BigInt(1));
      expect(input.amount).to.equal(BigInt(100000));
    });

    it('should create from Uint8Array', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);

      const input = new wasm.PlatformAddressInput(addressBytes, BigInt(0), BigInt(100000));
      expect(input).to.exist;
      expect(input.nonce).to.equal(BigInt(0));
      expect(input.amount).to.equal(BigInt(100000));
    });

    it('should handle zero nonce', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, BigInt(0), BigInt(100000));
      expect(input.nonce).to.equal(BigInt(0));
    });

    it('should handle large amounts', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const largeAmount = BigInt('10000000000000000'); // 10 quadrillion
      const input = new wasm.PlatformAddressInput(platformAddr, BigInt(1), largeAmount);
      expect(input.amount).to.equal(largeAmount);
    });

    it('should reject negative nonce', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      try {
        new wasm.PlatformAddressInput(platformAddr, BigInt(-1), BigInt(100000));
        expect.fail('Should have thrown error for negative nonce');
      } catch (error) {
        expect(error).to.exist;
      }
    });

    it('should reject nonce exceeding u32 max', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      try {
        new wasm.PlatformAddressInput(platformAddr, BigInt('4294967296'), BigInt(100000)); // u32::MAX + 1
        expect.fail('Should have thrown error for nonce exceeding u32 max');
      } catch (error) {
        expect(error.message).to.include('u32');
      }
    });
  });

  describe('getters', () => {
    it('should return the address', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, BigInt(1), BigInt(100000));
      const addr = input.address;
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2PKH');
    });

    it('should return the nonce', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, BigInt(42), BigInt(100000));
      expect(input.nonce).to.equal(BigInt(42));
    });

    it('should return the amount', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const input = new wasm.PlatformAddressInput(platformAddr, BigInt(1), BigInt(123456));
      expect(input.amount).to.equal(BigInt(123456));
    });
  });
});

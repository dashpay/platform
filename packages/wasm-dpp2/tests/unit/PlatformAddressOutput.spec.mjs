import getWasm from './helpers/wasm.js';

let wasm;

before(async () => {
  wasm = await getWasm();
});

describe('PlatformAddressOutput', () => {
  describe('construction', () => {
    it('should create from PlatformAddress object', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(500000));
      expect(output).to.exist;
      expect(output.amount).to.equal(BigInt(500000));
    });

    it('should create from bech32m address string', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);
      const bech32m = platformAddr.toBech32m('testnet');

      const output = new wasm.PlatformAddressOutput(bech32m, BigInt(90000));
      expect(output).to.exist;
      expect(output.amount).to.equal(BigInt(90000));
    });

    it('should create from Uint8Array', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);

      const output = new wasm.PlatformAddressOutput(addressBytes, BigInt(100000));
      expect(output).to.exist;
      expect(output.amount).to.equal(BigInt(100000));
    });

    it('should handle large amounts', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const largeAmount = BigInt('10000000000000000');
      const output = new wasm.PlatformAddressOutput(platformAddr, largeAmount);
      expect(output.amount).to.equal(largeAmount);
    });

    it('should handle zero amount', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(0));
      expect(output.amount).to.equal(BigInt(0));
    });

    it('should reject negative amount', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      try {
        new wasm.PlatformAddressOutput(platformAddr, BigInt(-1));
        expect.fail('Should have thrown error for negative amount');
      } catch (error) {
        expect(error).to.exist;
      }
    });
  });

  describe('getters', () => {
    it('should return the address', () => {
      const addressBytes = new Uint8Array([0x01, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(100000));
      const addr = output.address;
      expect(addr).to.exist;
      expect(addr.addressType).to.equal('P2SH');
    });

    it('should return the amount', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const platformAddr = wasm.PlatformAddress.fromBytes(addressBytes);

      const output = new wasm.PlatformAddressOutput(platformAddr, BigInt(999999));
      expect(output.amount).to.equal(BigInt(999999));
    });
  });
});

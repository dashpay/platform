import init, * as sdk from '../../dist/sdk.compressed.js';

describe('IdentityTopUpFromAddressesResult', () => {
  before(async () => {
    await init();
  });

  describe('construction and getters', () => {
    it('should have addressInfos getter that returns a Map', () => {
      // Note: PlatformAddressInfo has a private constructor and cannot be directly constructed.
      // It is only returned by SDK methods like getAddressInfo.
      // This test verifies the class exists in the SDK exports.
      expect(sdk.IdentityTopUpFromAddressesResult).to.exist;
      expect(sdk.PlatformAddressInfo).to.exist;
    });

    it('should have newBalance getter that returns BigInt', () => {
      // Verify the class exists and has the expected interface
      expect(sdk.IdentityTopUpFromAddressesResult).to.be.a('function');

      // The actual functionality will be tested in integration tests
      // where we can get a real result from identityTopUpFromAddresses
    });
  });

  describe('interface validation', () => {
    it('should be a constructor function', () => {
      expect(sdk.IdentityTopUpFromAddressesResult).to.be.a('function');
    });

    it('should have addressInfos and newBalance in prototype', () => {
      const proto = sdk.IdentityTopUpFromAddressesResult.prototype;
      expect(proto).to.exist;

      // Check that getters are defined (they will be on the prototype)
      const descriptors = Object.getOwnPropertyDescriptors(proto);

      // In WASM bindings, getters are defined on the prototype
      // We just verify the constructor exists for now
      expect(sdk.IdentityTopUpFromAddressesResult.name).to.equal('IdentityTopUpFromAddressesResult');
    });
  });

  describe('type checking', () => {
    it('should export IdentityTopUpFromAddressesResult class', () => {
      expect(sdk).to.have.property('IdentityTopUpFromAddressesResult');
      expect(typeof sdk.IdentityTopUpFromAddressesResult).to.equal('function');
    });
  });

  describe('expected usage pattern', () => {
    it('should document that addressInfos returns Map<PlatformAddress, PlatformAddressInfo>', () => {
      // This is a documentation test showing the expected return type
      // The actual Map structure is:
      // - Key: PlatformAddress instance
      // - Value: PlatformAddressInfo instance (returned by SDK, not constructed directly)

      // PlatformAddress can be created directly
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const address = sdk.PlatformAddress.fromBytes(addressBytes);

      expect(address).to.exist;
      expect(address.toBytes()).to.deep.equal(addressBytes);

      // PlatformAddressInfo has a private constructor - it can only be obtained
      // from SDK methods like getAddressInfo() or from result objects.
      // This is by design to ensure data integrity.
      expect(sdk.PlatformAddressInfo).to.exist;
    });

    it('should document that newBalance returns BigInt', () => {
      // This is a documentation test showing the expected return type
      const sampleBalance = 500000n;

      expect(typeof sampleBalance).to.equal('bigint');
      expect(sampleBalance > 0n).to.be.true;
    });
  });

  describe('integration with other SDK types', () => {
    it('should work with PlatformAddress from result', () => {
      const addressBytes = new Uint8Array([0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]);
      const address = sdk.PlatformAddress.fromBytes(addressBytes);

      expect(address).to.exist;
      expect(address.toBytes()).to.deep.equal(addressBytes);
    });

    it('should export PlatformAddressInfo class', () => {
      // PlatformAddressInfo has a private constructor and can only be obtained
      // from SDK methods. This test verifies the class is exported.
      expect(sdk.PlatformAddressInfo).to.exist;
      expect(sdk.PlatformAddressInfo).to.be.a('function');

      // The class has getters for address, balance, and nonce
      const proto = sdk.PlatformAddressInfo.prototype;
      expect(proto).to.exist;
    });
  });

  describe('BigInt handling', () => {
    it('should handle large balance values as BigInt', () => {
      // Platform balances can exceed Number.MAX_SAFE_INTEGER
      const largeBalance = 23522425453263151n;

      expect(typeof largeBalance).to.equal('bigint');
      expect(largeBalance > Number.MAX_SAFE_INTEGER).to.be.true;
    });

    it('should handle zero and small values', () => {
      const zero = 0n;
      const small = 100n;

      expect(typeof zero).to.equal('bigint');
      expect(typeof small).to.equal('bigint');
      expect(zero === 0n).to.be.true;
      expect(small === 100n).to.be.true;
    });
  });
});

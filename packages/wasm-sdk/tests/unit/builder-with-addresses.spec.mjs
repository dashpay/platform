import init, * as sdk from '../../dist/sdk.compressed.js';

// Test addresses (RFC 6761 reserved test domain - no network calls in unit tests)
const TEST_ADDRESS_1 = 'https://node-1.test:1443';
const TEST_ADDRESS_2 = 'https://node-2.test:1443';
const TEST_ADDRESS_3 = 'https://node-3.test:1443';

describe('WasmSdkBuilder.withAddresses()', () => {
  before(async () => {
    await init();
  });

  it('withAddresses() method exists', () => {
    expect(sdk.WasmSdkBuilder.withAddresses).to.be.a('function');
  });

  describe('valid configurations', () => {
    it('builds with single testnet address', async () => {
      const builder = sdk.WasmSdkBuilder.withAddresses(
        [TEST_ADDRESS_1],
        'testnet'
      );
      expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);

      const built = await builder.build();
      expect(built).to.be.an.instanceof(sdk.WasmSdk);
      expect(built.version()).to.be.a('number');
      expect(built.version()).to.be.greaterThan(0);
      built.free();
    });

    it('builds with multiple testnet addresses', async () => {
      const builder = sdk.WasmSdkBuilder.withAddresses(
        [
          TEST_ADDRESS_1,
          TEST_ADDRESS_2,
          TEST_ADDRESS_3
        ],
        'testnet'
      );
      expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      const built = await builder.build();
      expect(built).to.be.an.instanceof(sdk.WasmSdk);
      built.free();
    });

    it('builds with mainnet address', async () => {
      const builder = sdk.WasmSdkBuilder.withAddresses(
        [TEST_ADDRESS_1],
        'mainnet'
      );
      expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      const built = await builder.build();
      expect(built).to.be.an.instanceof(sdk.WasmSdk);
      built.free();
    });
  });

  describe('network validation', () => {
    it('rejects devnet', async () => {
      try {
        sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'devnet'
        );
        expect.fail('Should have thrown error for devnet');
      } catch (error) {
        expect(error.message).to.include('mainnet or testnet');
      }
    });

    it('rejects regtest', async () => {
      try {
        sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'regtest'
        );
        expect.fail('Should have thrown error for regtest');
      } catch (error) {
        expect(error.message).to.include('mainnet or testnet');
      }
    });

    it('rejects invalid network name', async () => {
      try {
        sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'invalid-network'
        );
        expect.fail('Should have thrown error for invalid network');
      } catch (error) {
        expect(error.message).to.include('mainnet or testnet');
      }
    });

    it('is case-insensitive for network names', async () => {
      const builder1 = sdk.WasmSdkBuilder.withAddresses(
        [TEST_ADDRESS_1],
        'TESTNET'
      );
      expect(builder1).to.be.an.instanceof(sdk.WasmSdkBuilder);

      const builder2 = sdk.WasmSdkBuilder.withAddresses(
        ['https://149.28.241.190:443'],
        'Mainnet'
      );
      expect(builder2).to.be.an.instanceof(sdk.WasmSdkBuilder);
    });
  });

  describe('address validation', () => {
    it('rejects URI without host', () => {
      try {
        sdk.WasmSdkBuilder.withAddresses(
          ['https://'],
          'testnet'
        );
        expect.fail('Should have thrown error for URI without host');
      } catch (error) {
        expect(error.message).to.include('Invalid URI');
        expect(error.message).to.include('https://');
      }
    });

    it('fails to build with empty address array', async () => {
      // Empty address array will panic when trying to build because
      // ConnectionPool requires non-zero addresses
      try {
        const builder = sdk.WasmSdkBuilder.withAddresses([], 'testnet');
        await builder.build();
        expect.fail('Should have panicked with empty address array');
      } catch (error) {
        // Expect panic or error when building with no addresses
        expect(error).to.exist();
      }
    });
  });

  describe('builder method chaining', () => {
    it('chains with withSettings()', async () => {
      let builder = sdk.WasmSdkBuilder.withAddresses(
        [TEST_ADDRESS_1],
        'testnet'
      );
      builder = builder.withSettings(5000, 10000, 3, false);
      expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      const built = await builder.build();
      expect(built).to.be.an.instanceof(sdk.WasmSdk);
      expect(built.version()).to.be.a('number');
      expect(built.version()).to.be.greaterThan(0);
      built.free();
    });

    it('chains with withVersion()', async () => {
      let builder = sdk.WasmSdkBuilder.withAddresses(
        [TEST_ADDRESS_1],
        'testnet'
      );
      builder = builder.withVersion(1);
      expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      const built = await builder.build();
      expect(built).to.be.an.instanceof(sdk.WasmSdk);
      expect(built.version()).to.be.a('number');
      expect(built.version()).to.equal(1);
      built.free();
    });

    it('chains multiple methods', async () => {
      let builder = sdk.WasmSdkBuilder.withAddresses(
        ['TEST_ADDRESS_1'],
        'testnet'
      );
      builder = builder
        .withVersion(1)
        .withProofs(true)
        .withSettings(5000, 10000, 3, false)
        .withLogs('debug');
      expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      const built = await builder.build();
      expect(built).to.be.an.instanceof(sdk.WasmSdk);
      expect(built.version()).to.be.a('number');
      expect(built.version()).to.equal(1);
      built.free();
    });
  });

});

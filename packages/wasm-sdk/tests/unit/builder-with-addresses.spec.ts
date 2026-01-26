import { expect } from './helpers/chai.ts';
import init, * as sdk from '../../dist/sdk.compressed.js';

// Test addresses (RFC 6761 reserved test domain - no network calls in unit tests)
const TEST_ADDRESS_1 = 'https://node-1.test:1443';
const TEST_ADDRESS_2 = 'https://node-2.test:1443';
const TEST_ADDRESS_3 = 'https://node-3.test:1443';

describe('WasmSdkBuilder', () => {
  before(async () => {
    await init();
  });

  describe('withAddresses()', () => {
    it('should be a function', () => {
      expect(sdk.WasmSdkBuilder.withAddresses).to.be.a('function');
    });

    describe('valid configurations', () => {
      it('should build with single testnet address', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'testnet',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);

        const built = await builder.build();
        expect(built).to.be.an.instanceof(sdk.WasmSdk);
        expect(built.version()).to.be.a('number');
        expect(built.version()).to.be.greaterThan(0);
        built.free();
      });

      it('should build with multiple testnet addresses', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          [
            TEST_ADDRESS_1,
            TEST_ADDRESS_2,
            TEST_ADDRESS_3,
          ],
          'testnet',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
        const built = await builder.build();
        expect(built).to.be.an.instanceof(sdk.WasmSdk);
        built.free();
      });

      it('should build with mainnet address', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'mainnet',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
        const built = await builder.build();
        expect(built).to.be.an.instanceof(sdk.WasmSdk);
        built.free();
      });

      it('should build with local address', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'local',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
        const built = await builder.build();
        expect(built).to.be.an.instanceof(sdk.WasmSdk);
        built.free();
      });
    });

    describe('network validation', () => {
      it('should reject devnet', async () => {
        try {
          sdk.WasmSdkBuilder.withAddresses(
            [TEST_ADDRESS_1],
            'devnet',
          );
          expect.fail('Should have thrown error for devnet');
        } catch (error) {
          expect(error.message).to.include('mainnet, testnet or local');
        }
      });

      it('should reject invalid network name', async () => {
        try {
          sdk.WasmSdkBuilder.withAddresses(
            [TEST_ADDRESS_1],
            'invalid-network',
          );
          expect.fail('Should have thrown error for invalid network');
        } catch (error) {
          expect(error.message).to.include('mainnet, testnet or local');
        }
      });

      it('should be case-insensitive for TESTNET', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'TESTNET',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      });

      it('should be case-insensitive for Mainnet', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          ['https://149.28.241.190:443'],
          'Mainnet',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      });

      it('should be case-insensitive for LOCAL', async () => {
        const builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'LOCAL',
        );
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
      });
    });

    describe('address validation', () => {
      it('should fail to build with empty address array', () => {
        try {
          sdk.WasmSdkBuilder.withAddresses(
            [],
            'testnet',
          );
          expect.fail('Should have thrown error for empty URI');
        } catch (error) {
          expect(error.message).to.equal('Addresses must be a non-empty array');
        }
      });

      it('should reject URI without host', () => {
        try {
          sdk.WasmSdkBuilder.withAddresses(
            ['https://'],
            'testnet',
          );
          expect.fail('Should have thrown error for URI without host');
        } catch (error) {
          expect(error.message).to.include('Invalid URI');
          expect(error.message).to.include('https://');
        }
      });
    });

    describe('builder method chaining', () => {
      it('should chain with withSettings()', async () => {
        let builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'testnet',
        );
        builder = builder.withSettings(5000, 10000, 3, false);
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
        const built = await builder.build();
        expect(built).to.be.an.instanceof(sdk.WasmSdk);
        expect(built.version()).to.be.a('number');
        expect(built.version()).to.be.greaterThan(0);
        built.free();
      });

      it('should chain with withVersion()', async () => {
        let builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'testnet',
        );
        builder = builder.withVersion(1);
        expect(builder).to.be.an.instanceof(sdk.WasmSdkBuilder);
        const built = await builder.build();
        expect(built).to.be.an.instanceof(sdk.WasmSdk);
        expect(built.version()).to.be.a('number');
        expect(built.version()).to.equal(1);
        built.free();
      });

      it('should chain multiple methods', async () => {
        let builder = sdk.WasmSdkBuilder.withAddresses(
          [TEST_ADDRESS_1],
          'testnet',
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
});

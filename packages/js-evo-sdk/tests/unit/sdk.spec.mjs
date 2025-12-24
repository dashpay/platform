import { EvoSDK } from '../../dist/evo-sdk.module.js';

// Test addresses (RFC 6761 reserved test domain - no network calls in unit tests)
const TEST_ADDRESS_1 = 'https://node-1.test:1443';
const TEST_ADDRESS_2 = 'https://node-2.test:1443';
const TEST_ADDRESS_3 = 'https://node-3.test:1443';
const TEST_ADDRESSES = [TEST_ADDRESS_1, TEST_ADDRESS_2, TEST_ADDRESS_3];

describe('EvoSDK', () => {
  it('exposes constructor and factories', () => {
    expect(EvoSDK).to.be.a('function');
    expect(EvoSDK.testnet).to.be.a('function');
    expect(EvoSDK.mainnet).to.be.a('function');
    expect(EvoSDK.testnetTrusted).to.be.a('function');
    expect(EvoSDK.mainnetTrusted).to.be.a('function');
    expect(EvoSDK.withAddresses).to.be.a('function');
  });

  it('fromWasm() marks instance as connected', () => {
    const wasmStub = { version: () => 1 };
    const sdk = EvoSDK.fromWasm(wasmStub);
    expect(sdk.isConnected).to.equal(true);
    expect(sdk.wasm).to.equal(wasmStub);
  });

  describe('EvoSDK.withAddresses()', () => {
    it('creates SDK instance with specific addresses', () => {
      const sdk = EvoSDK.withAddresses([TEST_ADDRESS_1], 'testnet');
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.isConnected).to.equal(false);
    });

    it('defaults to testnet when network not specified', () => {
      const sdk = EvoSDK.withAddresses([TEST_ADDRESS_1]);
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.isConnected).to.equal(false);
    });

    it('accepts mainnet network', () => {
      const sdk = EvoSDK.withAddresses([TEST_ADDRESS_2], 'mainnet');
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('mainnet');
      expect(sdk.isConnected).to.equal(false);
    });

    it('accepts multiple addresses', () => {
      const sdk = EvoSDK.withAddresses(TEST_ADDRESSES, 'testnet');
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.addresses).to.deep.equal(TEST_ADDRESSES);
    });

    it('accepts additional connection options', () => {
      const sdk = EvoSDK.withAddresses(
        [TEST_ADDRESS_1],
        'testnet',
        {
          version: 1,
          proofs: true,
          logs: 'info',
          settings: {
            connectTimeoutMs: 10000,
            timeoutMs: 30000,
            retries: 3,
            banFailedAddress: false,
          },
        },
      );
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.trusted).to.be.false();
      expect(sdk.options.addresses).to.deep.equal([TEST_ADDRESS_1]);
      expect(sdk.options.version).to.equal(1);
      expect(sdk.options.proofs).to.be.true();
      expect(sdk.options.logs).to.equal('info');
      expect(sdk.options.settings).to.exist();
      expect(sdk.options.settings.connectTimeoutMs).to.equal(10000);
      expect(sdk.options.settings.timeoutMs).to.equal(30000);
      expect(sdk.options.settings.retries).to.equal(3);
      expect(sdk.options.settings.banFailedAddress).to.be.false();
    });
  });

  describe('constructor with addresses option', () => {
    it('accepts addresses in options', () => {
      const sdk = new EvoSDK({
        addresses: [TEST_ADDRESS_1],
        network: 'testnet',
      });
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.trusted).to.be.false();
      expect(sdk.isConnected).to.equal(false);
    });

    it('works with testnet default', () => {
      const sdk = new EvoSDK({
        addresses: [TEST_ADDRESS_1],
      });
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.trusted).to.be.false();
    });

    it('works with mainnet', () => {
      const sdk = new EvoSDK({
        addresses: [TEST_ADDRESS_2],
        network: 'mainnet',
      });
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('mainnet');
      expect(sdk.options.trusted).to.be.false();
    });

    it('combines addresses with other options', () => {
      const sdk = new EvoSDK({
        addresses: [TEST_ADDRESS_1],
        network: 'testnet',
        version: 1,
        proofs: true,
        logs: 'debug',
        settings: {
          connectTimeoutMs: 5000,
          timeoutMs: 15000,
          retries: 5,
          banFailedAddress: true,
        },
      });
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.trusted).to.be.false();
      expect(sdk.options.addresses).to.deep.equal([TEST_ADDRESS_1]);
      expect(sdk.options.version).to.equal(1);
      expect(sdk.options.proofs).to.be.true();
      expect(sdk.options.logs).to.equal('debug');
      expect(sdk.options.settings).to.exist();
      expect(sdk.options.settings.connectTimeoutMs).to.equal(5000);
      expect(sdk.options.settings.timeoutMs).to.equal(15000);
      expect(sdk.options.settings.retries).to.equal(5);
      expect(sdk.options.settings.banFailedAddress).to.be.true();
    });

    it('prioritizes addresses over network presets when both provided', () => {
      // When addresses are provided, they should be used instead of default network addresses
      const sdk = new EvoSDK({
        addresses: [TEST_ADDRESS_3],
        network: 'testnet',
        trusted: true,
      });
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.addresses).to.deep.equal([TEST_ADDRESS_3]);
      expect(sdk.options.trusted).to.be.true();
    });

    it('withAddresses() and constructor with addresses produce equivalent SDKs', () => {
      const addresses = [TEST_ADDRESS_1];
      const options = { version: 1, proofs: true };

      const sdk1 = EvoSDK.withAddresses(addresses, 'testnet', options);
      const sdk2 = new EvoSDK({ addresses, network: 'testnet', ...options });

      expect(sdk1.options.addresses).to.deep.equal(sdk2.options.addresses);
      expect(sdk1.options.network).to.equal(sdk2.options.network);
      expect(sdk1.options.version).to.equal(sdk2.options.version);
      expect(sdk1.options.proofs).to.equal(sdk2.options.proofs);
    });
  });

  describe('factory methods for standard configurations', () => {
    it('testnet() creates testnet instance', () => {
      const sdk = EvoSDK.testnet();
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.trusted).to.be.false();
      expect(sdk.options.addresses).to.be.undefined();
      expect(sdk.isConnected).to.equal(false);
    });

    it('mainnet() creates mainnet instance', () => {
      const sdk = EvoSDK.mainnet();
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('mainnet');
      expect(sdk.options.trusted).to.be.false();
      expect(sdk.isConnected).to.equal(false);
    });

    it('testnetTrusted() creates trusted testnet instance', () => {
      const sdk = EvoSDK.testnetTrusted();
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('testnet');
      expect(sdk.options.trusted).to.be.true();
      expect(sdk.isConnected).to.equal(false);
    });

    it('mainnetTrusted() creates trusted mainnet instance', () => {
      const sdk = EvoSDK.mainnetTrusted();
      expect(sdk).to.be.instanceof(EvoSDK);
      expect(sdk.options.network).to.equal('mainnet');
      expect(sdk.options.trusted).to.be.true();
      expect(sdk.isConnected).to.equal(false);
    });

    it('factory methods accept connection options', () => {
      const sdk = EvoSDK.testnet({
        version: 1,
        proofs: false,
        logs: 'warn',
      });
      expect(sdk).to.be.instanceof(EvoSDK);
    });
  });
});

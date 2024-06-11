const EventEmitter = require('events');

const GrpcTransport = require('./transport/GrpcTransport/GrpcTransport');
const JsonRpcTransport = require('./transport/JsonRpcTransport/JsonRpcTransport');

const CoreMethodsFacade = require('./methods/core/CoreMethodsFacade');
const PlatformMethodsFacade = require('./methods/platform/PlatformMethodsFacade');

const createDAPIAddressProviderFromOptions = require('./dapiAddressProvider/createDAPIAddressProviderFromOptions');
const requestJsonRpc = require('./transport/JsonRpcTransport/requestJsonRpc');
const createGrpcTransportError = require('./transport/GrpcTransport/createGrpcTransportError');
const createJsonTransportError = require('./transport/JsonRpcTransport/createJsonTransportError');

const BlockHeadersProvider = require('./BlockHeadersProvider/BlockHeadersProvider');
const createBlockHeadersProviderFromOptions = require('./BlockHeadersProvider/createBlockHeadersProviderFromOptions');

const logger = require('./logger');

const EVENTS = {
  ERROR: 'error',
};

class DAPIClient extends EventEmitter {
  /**
   * @param {DAPIClientOptions} [options]
   */
  constructor(options = {}) {
    super();

    this.options = {
      network: 'testnet',
      timeout: 10000,
      retries: 5,
      blockHeadersProviderOptions: BlockHeadersProvider.defaultOptions,
      loggerOptions: {
        identifier: '',
        level: undefined,
      },
      ...options,
    };

    this.dapiAddressProvider = createDAPIAddressProviderFromOptions(this.options);

    const grpcTransport = new GrpcTransport(
      createDAPIAddressProviderFromOptions,
      this.dapiAddressProvider,
      createGrpcTransportError,
      this.options,
    );

    const jsonRpcTransport = new JsonRpcTransport(
      createDAPIAddressProviderFromOptions,
      requestJsonRpc,
      this.dapiAddressProvider,
      createJsonTransportError,
      this.options,
    );

    this.core = new CoreMethodsFacade(jsonRpcTransport, grpcTransport);
    this.platform = new PlatformMethodsFacade(grpcTransport);
    this.logger = logger.getForId(
      this.options.loggerOptions.identifier,
      this.options.loggerOptions.level,
    );

    this.initBlockHeadersProvider();
  }

  /**
   * @private
   */
  initBlockHeadersProvider() {
    this.blockHeadersProvider = createBlockHeadersProviderFromOptions(
      this.options,
      this.core,
      this.logger,
    );

    this.blockHeadersProvider.on(BlockHeadersProvider.EVENTS.ERROR, (e) => {
      this.emit(EVENTS.ERROR, e);
    });
  }

  /**
   * Close all open connections
   * @returns {Promise<void>}
   */
  async disconnect() {
    // Stop block headers provider
    await this.blockHeadersProvider.stop();

    // Stop masternode list provider
    if (this.dapiAddressProvider.smlProvider) {
      await this.dapiAddressProvider.smlProvider.unsubscribe();
    }
  }
}

DAPIClient.EVENTS = EVENTS;

/**
 * @typedef {DAPIClientOptions} DAPIClientOptions
 * @property {DAPIAddressProvider} [dapiAddressProvider]
 * @property {Array<RawDAPIAddress|DAPIAddress|string>} [dapiAddresses]
 * @property {Array<RawDAPIAddress|DAPIAddress|string>} [seeds]
 * @property {Array<RawDAPIAddress|DAPIAddress|string>} [dapiAddressesWhiteList]
 * @property {string|Network} [network=testnet]
 * @property {number} [timeout=2000]
 * @property {number} [retries=3]
 * @property {number} [baseBanTime=60000]
 * @property {boolean} [throwDeadlineExceeded]
 * @property {object} [loggerOptions]
 * @property {string} [loggerOptions.identifier]
 * @property {string} [loggerOptions.level]
 * @property {BlockHeadersProvider} [blockHeadersProvider]
 * @property {BlockHeadersProviderOptions} [blockHeadersProviderOptions]
 */

module.exports = DAPIClient;

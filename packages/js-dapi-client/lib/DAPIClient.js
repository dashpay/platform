const GrpcTransport = require('./transport/GrpcTransport');
const JsonRpcTransport = require('./transport/JsonRpcTransport/JsonRpcTransport');

const CoreMethodsFacade = require('./methods/core/CoreMethodsFacade');
const PlatformMethodsFacade = require('./methods/platform/PlatformMethodsFacade');

const createDAPIAddressProviderFromOptions = require('./dapiAddressProvider/createDAPIAddressProviderFromOptions');
const requestJsonRpc = require('./transport/JsonRpcTransport/requestJsonRpc');

class DAPIClient {
  /**
   * @param {DAPIClientOptions} [options]
   */
  constructor(options = {}) {
    this.options = {
      timeout: 2000,
      retries: 3,
      ...options,
    };

    this.dapiAddressProvider = createDAPIAddressProviderFromOptions(this.options);
    if (!this.dapiAddressProvider) {
      // Connect to evonet if no address options passed
      this.dapiAddressProvider = createDAPIAddressProviderFromOptions({ network: 'evonet' });
    }

    const grpcTransport = new GrpcTransport(
      createDAPIAddressProviderFromOptions,
      this.dapiAddressProvider,
      this.options,
    );

    const jsonRpcTransport = new JsonRpcTransport(
      createDAPIAddressProviderFromOptions,
      requestJsonRpc,
      this.dapiAddressProvider,
      this.options,
    );

    this.core = new CoreMethodsFacade(jsonRpcTransport, grpcTransport);
    this.platform = new PlatformMethodsFacade(grpcTransport);
  }
}

/**
 * @typedef {DAPIClientOptions} DAPIClientOptions
 * @property {DAPIAddressProvider} [dapiAddressProvider]
 * @property {Array<RawDAPIAddress|DAPIAddress|string>} [addresses]
 * @property {string[]|RawDAPIAddress[]} [seeds]
 * @property {string} [network=evonet]
 * @property {string} [networkType=testnet]
 * @property {number} [timeout=2000]
 * @property {number} [retries=3]
 * @property {number} [baseBanTime=60000]
 */

module.exports = DAPIClient;

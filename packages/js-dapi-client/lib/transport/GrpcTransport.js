const GrpcErrorCodes = require('@dashevo/grpc-common/lib/server/error/GrpcErrorCodes');

const MaxRetriesReachedError = require('./errors/MaxRetriesReachedError');
const NoAvailableAddressesForRetry = require('./errors/NoAvailableAddressesForRetry');

class GrpcTransport {
  /**
   * @param {createDAPIAddressProviderFromOptions} createDAPIAddressProviderFromOptions
   * @param {
   *    ListDAPIAddressProvider|
   *    SimplifiedMasternodeListDAPIAddressProvider|
   *    DAPIAddressProvider
   * } dapiAddressProvider
   * @param {DAPIClientOptions} globalOptions
   */
  constructor(createDAPIAddressProviderFromOptions, dapiAddressProvider, globalOptions) {
    this.createDAPIAddressProviderFromOptions = createDAPIAddressProviderFromOptions;
    this.dapiAddressProvider = dapiAddressProvider;
    this.globalOptions = globalOptions;

    this.lastUsedAddress = null;
  }

  /**
   * Make request to DAPI node
   *
   * @param {Function} ClientClass
   * @param {string} method
   * @param {object} requestMessage
   * @param {DAPIClientOptions} [options]
   *
   * @returns {Promise<object>}
   */
  async request(ClientClass, method, requestMessage, options = { }) {
    const dapiAddressProvider = this.createDAPIAddressProviderFromOptions(options)
      || this.dapiAddressProvider;

    const address = await dapiAddressProvider.getLiveAddress();

    // eslint-disable-next-line no-param-reassign
    options = {
      retries: this.globalOptions.retries,
      timeout: this.globalOptions.timeout,
      ...options,
    };

    const url = this.makeGrpcUrlFromAddress(address);
    const client = new ClientClass(url);

    try {
      const result = await client[method](requestMessage);

      this.lastUsedAddress = address;

      address.markAsLive();

      return result;
    } catch (error) {
      this.lastUsedAddress = address;

      if (error.code !== GrpcErrorCodes.DEADLINE_EXCEEDED
        && error.code !== GrpcErrorCodes.UNAVAILABLE
        && error.code !== GrpcErrorCodes.INTERNAL
        && error.code !== GrpcErrorCodes.CANCELLED
        && error.code !== GrpcErrorCodes.UNKNOWN) {
        throw error;
      }

      address.markAsBanned();

      if (options.retries === 0) {
        throw new MaxRetriesReachedError(error);
      }

      const hasAddresses = await dapiAddressProvider.hasLiveAddresses();
      if (!hasAddresses) {
        throw new NoAvailableAddressesForRetry(error);
      }

      return this.request(
        ClientClass,
        method,
        requestMessage,
        {
          ...options,
          retries: options.retries - 1,
        },
      );
    }
  }

  /**
   * Get last used address
   *
   * @returns {DAPIAddress|null}
   */
  getLastUsedAddress() {
    return this.lastUsedAddress;
  }

  /**
   *
   * Get gRPC url string
   *
   * @private
   * @param {DAPIAddress} address
   * @returns {string}
   */
  makeGrpcUrlFromAddress(address) {
    // For NodeJS Client
    if (typeof process !== 'undefined'
      && process.versions != null
      && process.versions.node != null) {
      return `${address.getHost()}:${address.getGrpcPort()}`;
    }

    // For gRPC-Web client
    const protocol = address.getHttpPort() === 443 ? 'https' : 'http';

    return `${protocol}://${address.getHost()}:${address.getHttpPort()}`;
  }
}

module.exports = GrpcTransport;

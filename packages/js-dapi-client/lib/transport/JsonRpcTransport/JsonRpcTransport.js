const MaxRetriesReachedError = require('../errors/MaxRetriesReachedError');
const NoAvailableAddressesForRetry = require('../errors/NoAvailableAddressesForRetry');
const NoAvailableAddresses = require('../errors/NoAvailableAddresses');

class JsonRpcTransport {
  /**
   * @param {createDAPIAddressProviderFromOptions} createDAPIAddressProviderFromOptions
   * @param {requestJsonRpc} requestJsonRpc
   * @param {
   *    ListDAPIAddressProvider|
   *    SimplifiedMasternodeListDAPIAddressProvider|
   *    DAPIAddressProvider
   * } dapiAddressProvider
   * @param {DAPIClientOptions} globalOptions
   */
  constructor(
    createDAPIAddressProviderFromOptions,
    requestJsonRpc,
    dapiAddressProvider,
    globalOptions,
  ) {
    this.createDAPIAddressProviderFromOptions = createDAPIAddressProviderFromOptions;
    this.requestJsonRpc = requestJsonRpc;
    this.dapiAddressProvider = dapiAddressProvider;
    this.globalOptions = globalOptions;

    this.lastUsedAddress = null;
  }

  /**
   * Make request to DAPI node
   *
   * @param {string} method
   * @param {object} [params]
   * @param {DAPIClientOptions} [options]
   *
   * @returns {Promise<object>}
   */
  async request(method, params = {}, options = {}) {
    const dapiAddressProvider = this.createDAPIAddressProviderFromOptions(options)
      || this.dapiAddressProvider;

    const address = await dapiAddressProvider.getLiveAddress();

    if (!address) {
      throw new NoAvailableAddresses();
    }

    // eslint-disable-next-line no-param-reassign
    options = {
      retries: this.globalOptions.retries,
      timeout: this.globalOptions.timeout,
      ...options,
    };

    const requestOptions = {};
    if (options.timeout !== undefined) {
      requestOptions.timeout = options.timeout;
    }

    try {
      const result = await this.requestJsonRpc(
        address.getHost(),
        address.getHttpPort(),
        method,
        params,
        requestOptions,
      );

      this.lastUsedAddress = address;

      address.markAsLive();

      return result;
    } catch (error) {
      this.lastUsedAddress = address;

      if (!['ECONNABORTED', 'ECONNREFUSED', 'ETIMEDOUT'].includes(error.code)
        && error.code !== -32603 && !(error.code >= -32000 && error.code <= -32099)) {
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
        method,
        params,
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
}

module.exports = JsonRpcTransport;

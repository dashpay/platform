const MaxRetriesReachedError = require('../errors/response/MaxRetriesReachedError');
const NoAvailableAddressesForRetryError = require('../errors/response/NoAvailableAddressesForRetryError');
const NoAvailableAddressesError = require('../errors/NoAvailableAddressesError');
const RetriableResponseError = require('../errors/response/RetriableResponseError');

class JsonRpcTransport {
  /**
   * @param {createDAPIAddressProviderFromOptions} createDAPIAddressProviderFromOptions
   * @param {requestJsonRpc} requestJsonRpc
   * @param {
   *    ListDAPIAddressProvider|
   *    SimplifiedMasternodeListDAPIAddressProvider|
   *    DAPIAddressProvider
   * } dapiAddressProvider
   * @param {createJsonTransportError} createJsonTransportError
   * @param {DAPIClientOptions} globalOptions
   */
  constructor(
    createDAPIAddressProviderFromOptions,
    requestJsonRpc,
    dapiAddressProvider,
    createJsonTransportError,
    globalOptions,
  ) {
    this.createDAPIAddressProviderFromOptions = createDAPIAddressProviderFromOptions;
    this.requestJsonRpc = requestJsonRpc;
    this.dapiAddressProvider = dapiAddressProvider;
    this.globalOptions = globalOptions;

    this.createJsonTransportError = createJsonTransportError;

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
      throw new NoAvailableAddressesError();
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

      if (error.code === undefined) {
        throw error;
      }

      address.markAsBanned();

      const responseError = this.createJsonTransportError(error, address);

      if (!(responseError instanceof RetriableResponseError)) {
        throw responseError;
      }

      if (options.retries === 0) {
        throw new MaxRetriesReachedError(responseError);
      }

      const hasAddresses = await dapiAddressProvider.hasLiveAddresses();
      if (!hasAddresses) {
        throw new NoAvailableAddressesForRetryError(responseError);
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

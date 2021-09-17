const MaxRetriesReachedError = require('../errors/response/MaxRetriesReachedError');
const NoAvailableAddressesForRetryError = require('../errors/response/NoAvailableAddressesForRetryError');
const NoAvailableAddressesError = require('../errors/NoAvailableAddressesError');
const TimeoutError = require('./errors/TimeoutError');
const RetriableResponseError = require('../errors/response/RetriableResponseError');

class GrpcTransport {
  /**
   * @param {createDAPIAddressProviderFromOptions} createDAPIAddressProviderFromOptions
   * @param {
   *    ListDAPIAddressProvider|
   *    SimplifiedMasternodeListDAPIAddressProvider|
   *    DAPIAddressProvider
   * } dapiAddressProvider
   * @param {createGrpcTransportError} createGrpcTransportError
   * @param {DAPIClientOptions} globalOptions
   */
  constructor(
    createDAPIAddressProviderFromOptions,
    dapiAddressProvider,
    createGrpcTransportError,
    globalOptions,
  ) {
    this.createDAPIAddressProviderFromOptions = createDAPIAddressProviderFromOptions;
    this.dapiAddressProvider = dapiAddressProvider;
    this.createGrpcTransportError = createGrpcTransportError;
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

    if (!address) {
      throw new NoAvailableAddressesError();
    }

    // eslint-disable-next-line no-param-reassign
    options = {
      retries: this.globalOptions.retries,
      timeout: this.globalOptions.timeout,
      ...options,
    };

    const url = this.makeGrpcUrlFromAddress(address);
    const client = new ClientClass(url);

    const requestOptions = {};
    if (options.timeout !== undefined) {
      requestOptions.deadline = new Date();
      requestOptions.deadline.setMilliseconds(
        requestOptions.deadline.getMilliseconds() + options.timeout,
      );
    }

    try {
      const result = await client[method](requestMessage, {}, requestOptions);

      this.lastUsedAddress = address;

      address.markAsLive();

      return result;
    } catch (error) {
      this.lastUsedAddress = address;

      // for unknown errors
      if (error.code === undefined) {
        throw error;
      }

      const responseError = this.createGrpcTransportError(error, address);

      if (!(responseError instanceof RetriableResponseError)) {
        throw responseError;
      }

      if (options.throwDeadlineExceeded && responseError instanceof TimeoutError) {
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
    let port = address.getHttpPort();

    // For NodeJS Client
    if (typeof process !== 'undefined'
      && process.versions != null
      && process.versions.node != null) {
      port = address.getGrpcPort();
    }

    const protocol = address.getHttpPort() === 443 ? 'https' : 'http';

    return `${protocol}://${address.getHost()}:${port}`;
  }
}

module.exports = GrpcTransport;

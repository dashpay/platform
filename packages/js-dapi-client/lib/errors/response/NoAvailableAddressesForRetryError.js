const ResponseError = require('./ResponseError');

class NoAvailableAddressesForRetryError extends ResponseError {
  /**
   * @param {number} code
   * @param {string} message
   * @param {module:grpc.Metadata} metadata
   * @param {DAPIAddress} dapiAddress
   */
  constructor(code, message, metadata, dapiAddress) {
    super(code, `No available addresses for retry: ${message}`, metadata, dapiAddress);
  }
}

module.exports = NoAvailableAddressesForRetryError;

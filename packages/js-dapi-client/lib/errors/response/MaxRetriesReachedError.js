const ResponseError = require('./ResponseError');

class MaxRetriesReachedError extends ResponseError {
  /**
   * @param {number} code
   * @param {string} message
   * @param {module:grpc.Metadata} metadata
   * @param {DAPIAddress} dapiAddress
   */
  constructor(code, message, metadata, dapiAddress) {
    super(code, `Max retries reached: ${message}`, metadata, dapiAddress);
  }
}

module.exports = MaxRetriesReachedError;

const DAPIClientError = require('../DAPIClientError');

class ResponseError extends DAPIClientError {
  /**
   *
   * @param {number} code
   * @param {string} message
   * @param {module:grpc.Metadata} metadata
   * @param {DAPIAddress} dapiAddress
   */
  constructor(code, message, metadata, dapiAddress) {
    super(message);

    this.code = code;
    this.metadata = metadata;
    this.dapiAddress = dapiAddress;
  }

  /**
   *
   * @returns {DAPIAddress}
   */
  getDapiAddress() {
    return this.dapiAddress;
  }

  /**
   *
   * @returns {number}
   */
  getCode() {
    return this.code;
  }

  /**
   *
   * @returns {module:grpc.Metadata}
   */
  getMetadata() {
    return this.metadata;
  }
}

module.exports = ResponseError;

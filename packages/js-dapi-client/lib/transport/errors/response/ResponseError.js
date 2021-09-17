const DAPIClientError = require('../../../errors/DAPIClientError');

class ResponseError extends DAPIClientError {
  /**
   * @param {number} code
   * @param {string} message
   * @param {object} data
   * @param {DAPIAddress} dapiAddress
   */
  constructor(code, message, data, dapiAddress) {
    super(message);

    this.code = code;
    this.data = data;
    this.dapiAddress = dapiAddress;
  }

  /**
   * @returns {DAPIAddress}
   */
  getDAPIAddress() {
    return this.dapiAddress;
  }

  /**
   * @returns {number}
   */
  getCode() {
    return this.code;
  }

  /**
   * @returns {object}
   */
  getData() {
    return this.data;
  }
}

module.exports = ResponseError;

const ServerError = require('../../errors/response/RetriableResponseError');

class InternalServerError extends ServerError {
  /**
   * @param {number} code
   * @param {string} message
   * @param {object} data
   * @param {DAPIAddress} dapiAddress
   */
  constructor(code, message, data, dapiAddress) {
    super(code, message, data, dapiAddress);

    // Replace current stack with remote stack from DAPI/Drive
    if (data.stack) {
      this.stack = `[REMOTE STACK] ${data.stack}`;
    }
  }
}

module.exports = InternalServerError;

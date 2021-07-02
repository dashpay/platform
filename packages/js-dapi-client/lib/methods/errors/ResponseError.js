const DAPIClientError = require('../../errors/DAPIClientError');

/**
 * @abstract
 */
class ResponseError extends DAPIClientError {
  /**
   * @param {number} code
   * @param {string} message
   */
  constructor(code, message) {
    super(message);

    this.code = code;
  }

  /**
   * @returns {number}
   */
  getCode() {
    return this.code;
  }
}

module.exports = ResponseError;

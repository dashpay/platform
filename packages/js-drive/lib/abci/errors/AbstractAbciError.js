const cbor = require('cbor');

const DriveError = require('../../errors/DriveError');

/**
 * @abstract
 */
class AbstractAbciError extends DriveError {
  /**
   *
   * @param {number} code
   * @param {string} message
   * @param {Object} data
   */
  constructor(code, message, data) {
    super(message);

    this.code = code;
    this.data = data;
  }

  /**
   * @returns {string}
   */
  getMessage() {
    return this.message;
  }

  /**
   * Get error code
   *
   * @returns {number}
   */
  getCode() {
    return this.code;
  }

  /**
   * Get error data
   *
   * @returns {Object}
   */
  getData() {
    return this.data;
  }

  /**
   * @returns {{code: number, info: string}}
   */
  getAbciResponse() {
    const info = {
      message: this.getMessage(),
      data: this.getData(),
    };

    return {
      code: this.getCode(),
      info: cbor.encode(info).toString('base64'),
    };
  }
}

module.exports = AbstractAbciError;

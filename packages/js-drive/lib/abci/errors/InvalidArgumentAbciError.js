const AbciError = require('./AbciError');

class InvalidArgumentAbciError extends AbciError {
  /**
   *
   * @param {string} message
   * @param {*} data
   */
  constructor(message, data = undefined) {
    super(
      AbciError.CODES.INVALID_ARGUMENT,
      message,
      data,
    );
  }
}

module.exports = InvalidArgumentAbciError;

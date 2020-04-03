const AbciError = require('./AbciError');

class NotFoundAbciError extends AbciError {
  /**
   *
   * @param {string} message
   * @param {*} data
   */
  constructor(message, data = undefined) {
    super(
      AbciError.CODES.NOT_FOUND,
      message,
      data,
    );
  }
}

module.exports = NotFoundAbciError;

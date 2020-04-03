const AbciError = require('./AbciError');

class InternalAbciError extends AbciError {
  /**
   *
   * @param {Error} error
   * @param {Object=undefined} data
   */
  constructor(error, data = undefined) {
    super(
      AbciError.CODES.INTERNAL,
      'Internal error',
      data,
    );

    this.error = error;
  }

  /**
   * Get error
   *
   * @return {Error}
   */
  getError() {
    return this.error;
  }
}

module.exports = InternalAbciError;

const AbciError = require('./AbciError');

class VerboseInternalAbciError extends AbciError {
  /**
   *
   * @param {InternalAbciError} error
   */
  constructor(error) {
    const originalError = error.getError();
    const [, errorPath] = originalError.stack.toString().split(/\r\n|\n/);

    const message = `${originalError.message} ${errorPath.trim()}`;

    super(
      error.getCode(),
      message,
      {
        stack: originalError.stack,
        data: error.getData(),
      },
    );
  }
}

module.exports = VerboseInternalAbciError;

const AbciError = require('./AbciError');

class VerboseInternalAbciError extends AbciError {
  /**
   *
   * @param {InternalAbciError} error
   */
  constructor(error) {
    const originalError = error.getError();

    let { message } = originalError;

    if (originalError.stack) {
      const [, errorPath] = originalError.stack.toString().split(/\r\n|\n/);

      message += ` ${errorPath.trim()}`;
    }

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

const InternalAbciError = require('./InternalAbciError');

class VerboseInternalAbciError extends InternalAbciError {
  /**
   *
   * @param {InternalAbciError} error
   */
  constructor(error) {
    const originalError = error.getError();
    let [, errorPath] = originalError.stack.toString().split(/\r\n|\n/);

    if (!errorPath) {
      errorPath = originalError.stack;
    }

    const message = `${originalError.message} ${errorPath.trim()}`;

    const data = error.getData() || {};
    data.stack = originalError.stack;

    super(
      originalError,
      data,
    );

    this.message = message;
  }
}

module.exports = VerboseInternalAbciError;

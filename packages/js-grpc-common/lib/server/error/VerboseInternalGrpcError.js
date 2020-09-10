const InternalGrpcError = require('./InternalGrpcError');

class VerboseInternalGrpcError extends InternalGrpcError {
  /**
   *
   * @param {InternalGrpcError} error
   */
  constructor(error) {
    const originalError = error.getError();
    const [, errorPath] = originalError.stack.toString().split(/\r\n|\n/);

    const message = `${originalError.message} ${errorPath.trim()}`;

    const rawMetadata = error.getRawMetadata() || {};
    rawMetadata.stack = originalError.stack;

    super(
      originalError,
      rawMetadata,
    );

    this.setMessage(message);
  }
}

module.exports = VerboseInternalGrpcError;

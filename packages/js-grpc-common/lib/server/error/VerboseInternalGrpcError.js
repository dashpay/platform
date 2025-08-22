const cbor = require('cbor');

const InternalGrpcError = require('./InternalGrpcError');

class VerboseInternalGrpcError extends InternalGrpcError {
  /**
   *
   * @param {InternalGrpcError} error
   */
  constructor(error) {
    const originalError = error.getError();

    let { message } = originalError;
    const rawMetadata = error.getRawMetadata() || {};

    if (originalError.stack) {
      let [, errorPath] = originalError.stack.toString()
        .split(/\r\n|\n/);

      if (!errorPath) {
        errorPath = originalError.stack;
      }

      message = `${message} ${errorPath.trim()}`;

      rawMetadata['stack-bin'] = cbor.encode(originalError.stack);
    }

    super(
      originalError,
      rawMetadata,
    );

    this.setMessage(message);
  }
}

module.exports = VerboseInternalGrpcError;

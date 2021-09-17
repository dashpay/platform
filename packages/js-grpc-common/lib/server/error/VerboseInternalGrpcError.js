const cbor = require('cbor');

const InternalGrpcError = require('./InternalGrpcError');

class VerboseInternalGrpcError extends InternalGrpcError {
  /**
   *
   * @param {InternalGrpcError} error
   */
  constructor(error) {
    const originalError = error.getError();
    let [, errorPath] = originalError.stack.toString().split(/\r\n|\n/);

    if (!errorPath) {
      errorPath = originalError.stack;
    }

    const message = `${originalError.message} ${errorPath.trim()}`;

    const rawMetadata = error.getRawMetadata() || {};
    rawMetadata['stack-bin'] = cbor.encode(originalError.stack);

    super(
      originalError,
      rawMetadata,
    );

    this.setMessage(message);
  }
}

module.exports = VerboseInternalGrpcError;

const GrpcError = require('./GrpcError');

class InternalGrpcError extends GrpcError {
  /**
   * @param {Error} error
   * @param {Object} [metadata]
   */
  constructor(error, metadata = undefined) {
    super(GrpcError.CODES.INTERNAL, 'Internal error', metadata);

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

module.exports = InternalGrpcError;

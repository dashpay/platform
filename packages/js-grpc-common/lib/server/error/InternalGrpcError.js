const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class InternalGrpcError extends GrpcError {
  /**
   * @param {Error} error
   * @param {Object} [metadata]
   */
  constructor(error, metadata = undefined) {
    super(GrpcErrorCodes.INTERNAL, 'Internal error', metadata);

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

const GrpcError = require('./GrpcError');

class InternalError extends GrpcError {
  /**
   * @param {Error} error
   */
  constructor(error) {
    super(GrpcError.CODES.INTERNAL, 'Internal error');

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

module.exports = InternalError;

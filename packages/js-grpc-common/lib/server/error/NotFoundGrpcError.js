const GrpcError = require('./GrpcError');

class NotFoundGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcError.CODES.NOT_FOUND, message, metadata);
  }
}

module.exports = NotFoundGrpcError;

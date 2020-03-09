const GrpcError = require('./GrpcError');

class DeadlineExceededGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcError.CODES.DEADLINE_EXCEEDED, message, metadata);
  }
}

module.exports = DeadlineExceededGrpcError;

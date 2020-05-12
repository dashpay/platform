const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class DeadlineExceededGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.DEADLINE_EXCEEDED, message, metadata);
  }
}

module.exports = DeadlineExceededGrpcError;

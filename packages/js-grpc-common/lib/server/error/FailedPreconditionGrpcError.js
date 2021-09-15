const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class FailedPreconditionGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.FAILED_PRECONDITION, message, metadata);
  }
}

module.exports = FailedPreconditionGrpcError;

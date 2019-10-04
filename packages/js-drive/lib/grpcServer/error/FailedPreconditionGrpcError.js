const GrpcError = require('./GrpcError');

class FailedPreconditionGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {grpc.Metadata} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcError.CODES.FAILED_PRECONDITION, `Failed precondition: ${message}`, metadata);
  }
}

module.exports = FailedPreconditionGrpcError;

const GrpcError = require('./GrpcError');

class ResourceExhaustedGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcError.CODES.RESOURCE_EXHAUSTED, message, metadata);
  }
}

module.exports = ResourceExhaustedGrpcError;

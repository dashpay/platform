const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class ResourceExhaustedGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.RESOURCE_EXHAUSTED, message, metadata);
  }
}

module.exports = ResourceExhaustedGrpcError;

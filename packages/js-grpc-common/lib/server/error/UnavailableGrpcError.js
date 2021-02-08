const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class UnavailableGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.UNAVAILABLE, message, metadata);
  }
}

module.exports = UnavailableGrpcError;

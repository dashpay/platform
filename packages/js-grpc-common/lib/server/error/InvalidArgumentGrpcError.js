const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class InvalidArgumentGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.INVALID_ARGUMENT, message, metadata);
  }
}

module.exports = InvalidArgumentGrpcError;

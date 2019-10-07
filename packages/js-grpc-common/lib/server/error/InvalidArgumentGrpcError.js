const GrpcError = require('./GrpcError');

class InvalidArgumentGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcError.CODES.INVALID_ARGUMENT, `Invalid argument: ${message}`, metadata);
  }
}

module.exports = InvalidArgumentGrpcError;

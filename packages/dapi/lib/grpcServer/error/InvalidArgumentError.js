const GrpcError = require('./GrpcError');

class InvalidArgumentError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcError.CODES.INVALID_ARGUMENT, `Invalid argument: ${message}`, metadata);
  }
}

module.exports = InvalidArgumentError;

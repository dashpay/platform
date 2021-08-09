const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class AlreadyExistsGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.ALREADY_EXISTS, message, metadata);
  }
}

module.exports = AlreadyExistsGrpcError;

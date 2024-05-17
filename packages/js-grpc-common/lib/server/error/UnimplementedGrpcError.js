const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class UnimplementedGrpcError extends GrpcError {
  /**
   * @param {string} message
   * @param {Object} [metadata]
   */
  constructor(message, metadata = undefined) {
    super(GrpcErrorCodes.UNIMPLEMENTED, message, metadata);
  }
}

module.exports = UnimplementedGrpcError;

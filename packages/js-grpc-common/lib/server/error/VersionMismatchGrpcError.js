const GrpcError = require('./GrpcError');
const GrpcErrorCodes = require('./GrpcErrorCodes');

class VersionMismatchGrpcError extends GrpcError {
  /**
   * @param {Object} [metadata]
   */
  constructor(metadata = undefined) {
    super(
      GrpcErrorCodes.VERSION_MISMATCH,
      'client and server versions mismatch',
      metadata,
    );
  }
}

module.exports = VersionMismatchGrpcError;

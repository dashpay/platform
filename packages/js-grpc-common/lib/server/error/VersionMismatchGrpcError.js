const GrpcError = require('./GrpcError');

class VersionMismatchGrpcError extends GrpcError {
  /**
   * @param {Object} [metadata]
   */
  constructor(metadata = undefined) {
    super(
      GrpcError.CODES.VERSION_MISMATCH,
      'client and server versions mismatch',
      metadata,
    );
  }
}

module.exports = VersionMismatchGrpcError;

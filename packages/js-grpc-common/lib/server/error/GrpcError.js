const convertObjectToMetadata = require('../../convertObjectToMetadata');

class GrpcError extends Error {
  /**
   * @param {string} message
   * @param {number} code
   * @param {Object} [metadata]
   */
  constructor(code, message, metadata = undefined) {
    super(message);

    this.code = code;

    if (metadata) {
      this.metadata = convertObjectToMetadata(metadata);
    }

    this.metadataObject = metadata;
  }

  /**
   * Get message
   *
   * @return {string}
   */
  getMessage() {
    return this.message;
  }

  /**
   * Get error code
   *
   * @return {number}
   */
  getCode() {
    return this.code;
  }

  /**
   * Get metadata
   *
   * @return {Object}
   */
  getMetadata() {
    return this.metadataObject;
  }
}

GrpcError.CODES = {
  CANCELLED: 1,
  UNKNOWN: 2,
  INVALID_ARGUMENT: 3,
  DEADLINE_EXCEEDED: 4,
  NOT_FOUND: 5,
  ALREADY_EXISTS: 6,
  PERMISSION_DENIED: 7,
  RESOURCE_EXHAUSTED: 8,
  FAILED_PRECONDITION: 9,
  ABORTED: 10,
  OUT_OF_RANGE: 11,
  UNIMPLEMENTED: 12,
  INTERNAL: 13,
  UNAVAILABLE: 14,
  DATA_LOSS: 15,
  UNAUTHENTICATED: 16,
  VERSION_MISMATCH: 100,
};

module.exports = GrpcError;

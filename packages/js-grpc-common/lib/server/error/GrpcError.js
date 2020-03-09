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
  INVALID_ARGUMENT: 3,
  DEADLINE_EXCEEDED: 4,
  NOT_FOUND: 5,
  RESOURCE_EXHAUSTED: 8,
  FAILED_PRECONDITION: 9,
  INTERNAL: 13,
};

module.exports = GrpcError;

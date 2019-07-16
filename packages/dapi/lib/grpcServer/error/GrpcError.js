class GrpcError extends Error {
  /**
   * @param {string} message
   * @param {number} code
   * @param {grpc.Metadata} [metadata]
   */
  constructor(code, message, metadata = undefined) {
    super(message);

    this.code = code;

    if (metadata) {
      this.metadata = metadata;
    }
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
    return this.metadata;
  }
}

GrpcError.CODES = {
  INTERNAL: 13,
  INVALID_ARGUMENT: 3,
};

module.exports = GrpcError;

const convertObjectToMetadata = require('../../convertObjectToMetadata');

class GrpcError extends Error {
  /**
   * @param {string} message
   * @param {number} code
   * @param {Object} [rawMetadata]
   */
  constructor(code, message, rawMetadata = undefined) {
    super(message);

    this.code = code;

    if (rawMetadata) {
      this.metadata = convertObjectToMetadata(rawMetadata);
    }

    this.rawMetadata = rawMetadata;
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
  getRawMetadata() {
    return this.rawMetadata;
  }

  /**
   *
   * @param {Object} rawMetadata
   * @return {GrpcError}
   */
  setRawMetadata(rawMetadata) {
    this.metadata = convertObjectToMetadata(rawMetadata);

    this.rawMetadata = rawMetadata;

    return this;
  }

  /**
   *
   * @param {string} message
   * @return {GrpcError}
   */
  setMessage(message) {
    this.message = message;

    return this;
  }
}

module.exports = GrpcError;

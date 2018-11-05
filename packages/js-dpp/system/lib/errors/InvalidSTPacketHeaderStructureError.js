class InvalidSTPacketStructureHeaderError extends Error {
  /**
   * @param {Object[]} errors
   * @param {Object} rawSTPacketHeader
   */
  constructor(errors, rawSTPacketHeader) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid ST Packet Header structure';

    this.errors = errors;
    this.rawSTPacketHeader = rawSTPacketHeader;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get validation errors
   *
   * @return {Object[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get raw ST Packet Header
   *
   * @return {Object}
   */
  getRawSTPacketHeader() {
    return this.rawSTPacketHeader;
  }
}

module.exports = InvalidSTPacketStructureHeaderError;

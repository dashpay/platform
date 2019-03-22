class InvalidSTPacketStructureError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {RawSTPacketHeader} rawSTPacketHeader
   */
  constructor(errors, rawSTPacketHeader) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid ST Packet Header';

    this.errors = errors;
    this.rawSTPacketHeader = rawSTPacketHeader;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get validation errors
   *
   * @return {ConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }

  /**
   * Get raw ST Packet Header
   *
   * @return {RawSTPacketHeader}
   */
  getRawSTPacketHeader() {
    return this.rawSTPacketHeader;
  }
}

module.exports = InvalidSTPacketStructureError;

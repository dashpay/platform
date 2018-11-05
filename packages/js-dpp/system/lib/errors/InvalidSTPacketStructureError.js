class InvalidSTPacketStructureError extends Error {
  /**
   * @param {Object[]} errors
   * @param {Object} rawStPacket
   */
  constructor(errors, rawStPacket) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid ST Packet structure';

    this.errors = errors;
    this.rawStPacket = rawStPacket;

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
   * Get raw ST Packet
   *
   * @return {Object}
   */
  getRqwStPacket() {
    return this.rawStPacket;
  }
}

module.exports = InvalidSTPacketStructureError;

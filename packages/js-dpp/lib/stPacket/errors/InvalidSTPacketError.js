class InvalidSTPacketError extends Error {
  /**
   * @param {ConsensusError[]} errors
   * @param {Object} rawSTPacket
   */
  constructor(errors, rawSTPacket) {
    super();

    this.name = this.constructor.name;
    this.message = 'Invalid ST Packet';

    this.errors = errors;
    this.rawSTPacket = rawSTPacket;

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
   * Get raw ST Packet
   *
   * @return {Object}
   */
  getRawSTPacket() {
    return this.rawSTPacket;
  }
}

module.exports = InvalidSTPacketError;

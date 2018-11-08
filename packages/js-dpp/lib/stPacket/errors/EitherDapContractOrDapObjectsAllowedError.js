class EitherDapContractOrDapObjectsAllowedError extends Error {
  /**
   * @param {STPacket} stPacket
   */
  constructor(stPacket) {
    super();

    this.name = this.constructor.name;
    this.message = 'Either DapContract Or DapObjects is allowed in the same packet';

    this.stPacket = stPacket;

    if (Error.captureStackTrace) {
      Error.captureStackTrace(this, this.constructor);
    }
  }

  /**
   * Get ST Packet
   *
   * @return {STPacket}
   */
  getSTPacket() {
    return this.rawSTPacketHeader;
  }
}

module.exports = EitherDapContractOrDapObjectsAllowedError;

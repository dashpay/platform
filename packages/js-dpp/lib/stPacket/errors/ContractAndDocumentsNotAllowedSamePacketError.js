class ContractAndDocumentsNotAllowedSamePacketError extends Error {
  /**
   * @param {STPacket} stPacket
   */
  constructor(stPacket) {
    super();

    this.name = this.constructor.name;
    this.message = 'Either Contract Or Documents is allowed in the same packet';

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
    return this.stPacket;
  }
}

module.exports = ContractAndDocumentsNotAllowedSamePacketError;

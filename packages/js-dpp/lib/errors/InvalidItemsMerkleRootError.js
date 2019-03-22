const ConsensusError = require('./ConsensusError');

class InvalidItemsMerkleRootError extends ConsensusError {
  /**
   * @param {RawSTPacket} rawSTPacket
   */
  constructor(rawSTPacket) {
    super('Invalid ST Packet\'s itemsMerkleRoot');

    this.rawSTPacket = rawSTPacket;
  }

  /**
   * Get raw ST Packet
   *
   * @return {RawSTPacket}
   */
  getRawSTPacket() {
    return this.rawSTPacket;
  }
}

module.exports = InvalidItemsMerkleRootError;

const ConsensusError = require('./ConsensusError');

class InvalidItemsMerkleRootError extends ConsensusError {
  /**
   * @param {Object} rawSTPacket
   */
  constructor(rawSTPacket) {
    super('Invalid ST Packet\'s itemsMerkleRoot');

    this.rawSTPacket = rawSTPacket;
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

module.exports = InvalidItemsMerkleRootError;

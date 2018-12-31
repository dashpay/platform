const ConsensusError = require('./ConsensusError');

class InvalidItemsHashError extends ConsensusError {
  /**
   * @param {Object} rawSTPacket
   */
  constructor(rawSTPacket) {
    super('Invalid ST Packet\'s itemsHash');

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

module.exports = InvalidItemsHashError;

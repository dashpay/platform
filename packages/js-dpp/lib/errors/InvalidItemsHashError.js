const ConsensusError = require('./ConsensusError');

class InvalidItemsHashError extends ConsensusError {
  /**
   * @param {RawSTPacket} rawSTPacket
   */
  constructor(rawSTPacket) {
    super('Invalid ST Packet\'s itemsHash');

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

module.exports = InvalidItemsHashError;

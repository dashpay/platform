const ConsensusError = require('./ConsensusError');

class InvalidSTPacketHashError extends ConsensusError {
  /**
   * @param {STPacket} stPacket
   * @param {Transaction} stateTransition
   */
  constructor(stPacket, stateTransition) {
    super('Invalid ST Packet hash in State Transition payload');

    this.stPacket = stPacket;
    this.stateTransition = stateTransition;
  }

  /**
   * Get ST Packet
   *
   * @return {STPacket}
   */
  getSTPacket() {
    return this.stPacket;
  }

  /**
   * Get State Transition
   *
   * @return {Transaction}
   */
  getStateTransition() {
    return this.stateTransition;
  }
}

module.exports = InvalidSTPacketHashError;

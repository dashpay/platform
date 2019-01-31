class InvalidSTPacketDataError extends Error {
  /**
   *
   * @param {STPacket} stPacket
   * @param {StateTransition} stateTransition
   * @param {ConsensusError[]} errors
   */
  constructor(stPacket, stateTransition, errors) {
    super();

    this.message = 'Invalid ST Packet data';
    this.name = this.constructor.name;

    this.stPacket = stPacket;
    this.stateTransition = stateTransition;
    this.errors = errors;

    Error.captureStackTrace(this, this.constructor);
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
   * @return {StateTransition}
   */
  getStateTransition() {
    return this.stateTransition;
  }

  /**
   * Get errors
   *
   * @return {ConsensusError[]}
   */
  getErrors() {
    return this.errors;
  }
}

module.exports = InvalidSTPacketDataError;

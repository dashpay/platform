const ConsensusError = require('./ConsensusError');

class InvalidDPContractError extends ConsensusError {
  /**
   * @param {DPContract} dpContract
   * @param {Object} rawSTPacket
   */
  constructor(dpContract, rawSTPacket) {
    super('Invalid DP Contract for ST Packet validation');

    this.dpContract = dpContract;
    this.rawSTPacket = rawSTPacket;
  }

  /**
   * Get contract ID
   *
   * @return {DPContract}
   */
  getDPContract() {
    return this.dpContract;
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

module.exports = InvalidDPContractError;

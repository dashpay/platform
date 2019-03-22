const ConsensusError = require('./ConsensusError');

class InvalidContractError extends ConsensusError {
  /**
   * @param {Contract} contract
   * @param {Object} rawSTPacket
   */
  constructor(contract, rawSTPacket) {
    super('Invalid Contract for ST Packet validation');

    this.contract = contract;
    this.rawSTPacket = rawSTPacket;
  }

  /**
   * Get contract ID
   *
   * @return {Contract}
   */
  getContract() {
    return this.contract;
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

module.exports = InvalidContractError;

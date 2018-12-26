const ConsensusError = require('./ConsensusError');

class InvalidDapContractError extends ConsensusError {
  /**
   * @param {DapContract} dapContract
   * @param {Object} rawSTPacket
   */
  constructor(dapContract, rawSTPacket) {
    super('Invalid DAP Contract for ST Packet validation');

    this.dapContract = dapContract;
    this.rawSTPacket = rawSTPacket;
  }

  /**
   * Get contract ID
   *
   * @return {DapContract}
   */
  getDapContract() {
    return this.dapContract;
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

module.exports = InvalidDapContractError;

const ConsensusError = require('./ConsensusError');

class InvalidSTPacketContractIdError extends ConsensusError {
  /**
   * @param {string} dpContractId
   * @param {DPContract} dpContract
   */
  constructor(dpContractId, dpContract) {
    super('ST Packet\'s contractId should be equal to DP Contract ID');

    this.dpContractId = dpContractId;
    this.dpContract = dpContract;
  }

  /**
   * Get contract ID
   *
   * @return {string}
   */
  getDPContractId() {
    return this.dpContractId;
  }

  /**
   * Get DP Contract
   *
   * @return {DPContract}
   */
  getDPContract() {
    return this.dpContract;
  }
}

module.exports = InvalidSTPacketContractIdError;

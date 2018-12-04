const ConsensusError = require('./ConsensusError');

class InvalidSTPacketContractIdError extends ConsensusError {
  /**
   * @param {string} contractId
   * @param {DapContract} dapContract
   */
  constructor(contractId, dapContract) {
    super('ST Packet\'s contractId should be equal to DapContract ID');

    this.contractId = contractId;
    this.dapContract = dapContract;
  }

  /**
   * Get contract ID
   *
   * @return {string}
   */
  getContractId() {
    return this.contractId;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract}
   */
  getDapContractId() {
    return this.dapContract;
  }
}

module.exports = InvalidSTPacketContractIdError;

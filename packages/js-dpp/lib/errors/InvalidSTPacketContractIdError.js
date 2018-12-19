const ConsensusError = require('./ConsensusError');

class InvalidSTPacketContractIdError extends ConsensusError {
  /**
   * @param {string} dapContractId
   * @param {DapContract} dapContract
   */
  constructor(dapContractId, dapContract) {
    super('ST Packet\'s contractId should be equal to DapContract ID');

    this.dapContractId = dapContractId;
    this.dapContract = dapContract;
  }

  /**
   * Get contract ID
   *
   * @return {string}
   */
  getDapContractId() {
    return this.dapContractId;
  }

  /**
   * Get Dap Contract
   *
   * @return {DapContract}
   */
  getDapContract() {
    return this.dapContract;
  }
}

module.exports = InvalidSTPacketContractIdError;

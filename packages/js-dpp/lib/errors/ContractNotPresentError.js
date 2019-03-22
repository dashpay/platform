const ConsensusError = require('./ConsensusError');

class ContractNotPresentError extends ConsensusError {
  /**
   * @param {string} contractId
   */
  constructor(contractId) {
    super('Contract is not present with contract ID specified in ST Packet');

    this.contractId = contractId;
  }

  /**
   * Get contract ID
   *
   * @return {string}
   */
  getContractId() {
    return this.contractId;
  }
}

module.exports = ContractNotPresentError;

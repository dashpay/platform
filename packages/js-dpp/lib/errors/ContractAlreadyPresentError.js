const ConsensusError = require('./ConsensusError');

class ContractAlreadyPresentError extends ConsensusError {
  /**
   * @param {Contract} contract
   */
  constructor(contract) {
    super('Contract is already present');

    this.contract = contract;
  }

  /**
   * Get Contract
   *
   * @return {Contract}
   */
  getContract() {
    return this.contract;
  }
}

module.exports = ContractAlreadyPresentError;

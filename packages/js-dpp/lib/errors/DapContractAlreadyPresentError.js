const ConsensusError = require('./ConsensusError');

class DapContractAlreadyPresentError extends ConsensusError {
  /**
   * @param {DapContract} dapContract
   */
  constructor(dapContract) {
    super('Dap Contract is already present');

    this.dapContract = dapContract;
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

module.exports = DapContractAlreadyPresentError;

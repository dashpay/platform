const ConsensusError = require('./ConsensusError');

class DPContractAlreadyPresentError extends ConsensusError {
  /**
   * @param {DPContract} dpContract
   */
  constructor(dpContract) {
    super('DP Contract is already present');

    this.dpContract = dpContract;
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

module.exports = DPContractAlreadyPresentError;

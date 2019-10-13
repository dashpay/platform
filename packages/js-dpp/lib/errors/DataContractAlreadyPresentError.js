const ConsensusError = require('./ConsensusError');

class DataContractAlreadyPresentError extends ConsensusError {
  /**
   * @param {DataContract} dataContract
   */
  constructor(dataContract) {
    super('Data Contract is already present');

    this.dataContract = dataContract;
  }

  /**
   * Get Data Contract
   *
   * @return {DataContract}
   */
  getDataContract() {
    return this.dataContract;
  }
}

module.exports = DataContractAlreadyPresentError;

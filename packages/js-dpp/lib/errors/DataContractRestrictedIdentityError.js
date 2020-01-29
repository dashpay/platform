const ConsensusError = require('./ConsensusError');

class DataContractRestrictedIdentityError extends ConsensusError {
  /**
   * @param {DataContract} dataContract
   */
  constructor(dataContract) {
    super('The identity is not allowed to register contracts');

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

module.exports = DataContractRestrictedIdentityError;

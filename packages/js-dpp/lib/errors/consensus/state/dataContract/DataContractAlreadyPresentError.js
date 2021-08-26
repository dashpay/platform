const AbstractStateError = require('../AbstractStateError');

class DataContractAlreadyPresentError extends AbstractStateError {
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

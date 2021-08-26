const AbstractBasicError = require('../AbstractBasicError');

class DataContractNotPresentError extends AbstractBasicError {
  /**
   * @param {Identifier|Buffer} dataContractId
   */
  constructor(dataContractId) {
    super('Data Contract with specified ID is not present');

    this.dataContractId = dataContractId;
  }

  /**
   * Get Data Contract ID
   *
   * @return {Identifier|Buffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = DataContractNotPresentError;

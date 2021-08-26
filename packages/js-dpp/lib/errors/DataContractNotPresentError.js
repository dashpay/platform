const DPPError = require('./DPPError');

class DataContractNotPresentError extends DPPError {
  /**
   * @param {Identifier} dataContractId
   */
  constructor(dataContractId) {
    super('Data Contract is not present');

    this.dataContractId = dataContractId;
  }

  /**
   * Get Data Contract ID
   *
   * @return {Identifier}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = DataContractNotPresentError;

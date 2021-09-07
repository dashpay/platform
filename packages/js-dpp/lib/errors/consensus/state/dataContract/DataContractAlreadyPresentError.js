const AbstractStateError = require('../AbstractStateError');
const Identifier = require('../../../../identifier/Identifier');

class DataContractAlreadyPresentError extends AbstractStateError {
  /**
   * @param {Buffer} dataContractId
   */
  constructor(dataContractId) {
    super(`Data Contract ${Identifier.from(dataContractId)} is already present`);

    this.dataContractId = dataContractId;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get Data Contract ID
   *
   * @return {Buffer}
   */
  getDataContractId() {
    return this.dataContractId;
  }
}

module.exports = DataContractAlreadyPresentError;

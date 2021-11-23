const AbstractStateError = require('../AbstractStateError');

class InvalidDataContractBaseDataError extends AbstractStateError {
  /**
   * @param {Object} oldBaseDataContract
   * @param {Object} newBaseDataContract
   */
  constructor(oldBaseDataContract, newBaseDataContract) {
    super('Only $defs, $version and documents fields are allowed to be updated');

    this.oldBaseDataContract = oldBaseDataContract;
    this.newBaseDataContract = newBaseDataContract;

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }

  /**
   * Get old base data contract
   * @returns {Object}
   */
  getOldBaseDataContract() {
    return this.oldBaseDataContract;
  }

  /**
   * Get new base data contract
   * @returns {Object}
   */
  getNewBaseDataContract() {
    return this.newBaseDataContract;
  }
}

module.exports = InvalidDataContractBaseDataError;

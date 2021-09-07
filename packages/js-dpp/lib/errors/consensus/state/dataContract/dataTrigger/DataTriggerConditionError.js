const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerConditionError extends AbstractDataTriggerError {
  /**
   * @param {Buffer} dataContractId
   * @param {Buffer} documentTransitionId
   * @param {string} message
   */
  constructor(dataContractId, documentTransitionId, message) {
    super(dataContractId, documentTransitionId, message);

    // eslint-disable-next-line prefer-rest-params
    this.setConstructorArguments(arguments);
  }
}

module.exports = DataTriggerConditionError;

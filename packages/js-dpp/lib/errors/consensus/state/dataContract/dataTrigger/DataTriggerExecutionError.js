const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerExecutionError extends AbstractDataTriggerError {
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

  /**
   * Set internal error
   *
   * @param {Error} error
   */
  setExecutionError(error) {
    this.executionError = error;
  }

  /**
   * Return internal error
   *
   * @return {Error}
   */
  getExecutionError() {
    return this.executionError;
  }
}

module.exports = DataTriggerExecutionError;

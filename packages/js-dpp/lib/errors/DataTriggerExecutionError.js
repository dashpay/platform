const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerExecutionError extends AbstractDataTriggerError {
  /**
   * @param {DataTrigger} dataTrigger
   * @param {DataContract} dataContract
   * @param {string} userId
   * @param {Error} error
   */
  constructor(dataTrigger, dataContract, userId, error) {
    super(error.message, dataContract, userId);

    this.error = error;
    this.dataTrigger = dataTrigger;
  }

  /**
   * Return internal error instance
   *
   * @return {Error}
   */
  getError() {
    return this.error;
  }

  /**
   * Get data trigger
   *
   * @return {DataTrigger}
   */
  getDataTrigger() {
    return this.dataTrigger;
  }
}

module.exports = DataTriggerExecutionError;

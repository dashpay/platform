const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerExecutionError extends AbstractDataTriggerError {
  /**
   * @param {DataTrigger} dataTrigger
   * @param {DataContract} dataContract
   * @param {Buffer} ownerId
   * @param {Error} error
   */
  constructor(dataTrigger, dataContract, ownerId, error) {
    super(error.message, dataContract, ownerId);

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

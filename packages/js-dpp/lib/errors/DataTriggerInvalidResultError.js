const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerInvalidResultError extends AbstractDataTriggerError {
  /**
   * @param {DataTrigger} dataTrigger
   * @param {DataContract} dataContract
   * @param {string} userId
   */
  constructor(dataTrigger, dataContract, userId) {
    super('Data trigger have not returned any result', dataContract, userId);

    this.dataTrigger = dataTrigger;
  }

  /**
   * Get data trigger
   *
   * @returns {DataTrigger}
   */
  getDataTrigger() {
    return this.dataTrigger;
  }
}

module.exports = DataTriggerInvalidResultError;

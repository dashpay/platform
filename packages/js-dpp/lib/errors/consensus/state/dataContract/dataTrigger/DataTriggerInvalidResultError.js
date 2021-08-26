const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerInvalidResultError extends AbstractDataTriggerError {
  /**
   * @param {DataTrigger} dataTrigger
   * @param {DataContract} dataContract
   * @param {Identifier|Buffer} ownerId
   */
  constructor(dataTrigger, dataContract, ownerId) {
    super('Data trigger have not returned any result', dataContract, ownerId);

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

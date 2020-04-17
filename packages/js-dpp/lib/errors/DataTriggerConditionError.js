const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerConditionError extends AbstractDataTriggerError {
  /**
   * @param {Document} document
   * @param {DataContract} dataContract
   * @param {string} ownerId
   * @param {string} message
   */
  constructor(document, dataContract, ownerId, message) {
    super(message, dataContract, ownerId);

    this.document = document;
  }

  /**
   * Get document
   *
   * @returns {Document}
   */
  getDocument() {
    return this.document;
  }
}

module.exports = DataTriggerConditionError;

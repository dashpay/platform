const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerConditionError extends AbstractDataTriggerError {
  /**
   * @param {Document} document
   * @param {DataContract} dataContract
   * @param {string} userId
   * @param {string} message
   */
  constructor(document, dataContract, userId, message) {
    super(message, dataContract, userId);

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

const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerConditionError extends AbstractDataTriggerError {
  /**
   * @param {
   *   DocumentCreateTransition|DocumentReplaceTranition|DocumentDeleteTransition
   * } documentTransition
   * @param {DataContract} dataContract
   * @param {string} ownerId
   * @param {string} message
   */
  constructor(documentTransition, dataContract, ownerId, message) {
    super(message, dataContract, ownerId);

    this.documentTransition = documentTransition;
  }

  /**
   * Get document transition
   *
   * @returns {
   *   DocumentCreateTransition|DocumentReplaceTranition|DocumentDeleteTransition
   * }
   */
  getDocumentTransiton() {
    return this.documentTransition;
  }
}

module.exports = DataTriggerConditionError;

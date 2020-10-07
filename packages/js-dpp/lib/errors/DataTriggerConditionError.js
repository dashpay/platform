const AbstractDataTriggerError = require('./AbstractDataTriggerError');

class DataTriggerConditionError extends AbstractDataTriggerError {
  /**
   * @param {
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * } documentTransition
   * @param {DataContract} dataContract
   * @param {EncodedBuffer} ownerId
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
   *   DocumentCreateTransition|DocumentReplaceTransition|DocumentDeleteTransition
   * }
   */
  getDocumentTransition() {
    return this.documentTransition;
  }
}

module.exports = DataTriggerConditionError;

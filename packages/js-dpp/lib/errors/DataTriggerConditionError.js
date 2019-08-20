const ConsensusError = require('./ConsensusError');

class DataTriggerConditionError extends ConsensusError {
  /**
   * @param {Document} document
   * @param {DataTriggerExecutionContext} context
   * @param {string} message
   */
  constructor(document, context, message) {
    super(message);

    this.document = document;
    this.context = context;
  }

  /**
   * Get document
   *
   * @returns {Document}
   */
  getDocument() {
    return this.document;
  }

  /**
   * Get data trigger execution context
   *
   * @return {DataTriggerExecutionContext}
   */
  getContext() {
    return this.context;
  }
}

module.exports = DataTriggerConditionError;

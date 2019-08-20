const DataTriggerExecutionResult = require('./DataTriggerExecutionResult');
const DataTriggerExecutionError = require('../errors/DataTriggerExecutionError');
const DataTriggerInvalidResultError = require('../errors/DataTriggerInvalidResultError');

class DataTrigger {
  /**
   * @param {string} contractId
   * @param {number} documentType
   * @param {number} documentAction
   * @param {function(Document, DataTriggerExecutionContext):DataTriggerExecutionResult} trigger
   */
  constructor(contractId, documentType, documentAction, trigger) {
    this.contractId = contractId;
    this.documentType = documentType;
    this.documentAction = documentAction;
    this.trigger = trigger;
  }

  /**
   * Check this trigger is matching for specified data
   *
   * @param {string} contractId
   * @param {number} documentType
   * @param {number} documentAction
   *
   * @return {boolean}
   */
  isMatchingTriggerForData(contractId, documentType, documentAction) {
    return this.contractId === contractId
      && this.documentType === documentType
      && this.documentAction === documentAction;
  }

  /**
   * Execute data trigger
   *
   * @param {Document} document
   * @param {DataTriggerExecutionContext} context
   *
   * @returns {Promise<DataTriggerExecutionResult>}
   */
  async execute(document, context) {
    let result = new DataTriggerExecutionResult();

    try {
      result = await this.trigger(document, context);
    } catch (e) {
      result.addError(
        new DataTriggerExecutionError(
          this, context, e,
        ),
      );
    }

    if (!(result instanceof DataTriggerExecutionResult)) {
      result = new DataTriggerExecutionResult();
      result.addError(
        new DataTriggerInvalidResultError(
          this, context,
        ),
      );
    }

    return result;
  }
}

module.exports = DataTrigger;

const DataTriggerExecutionResult = require('./DataTriggerExecutionResult');
const DataTriggerExecutionError = require('../errors/DataTriggerExecutionError');
const DataTriggerInvalidResultError = require('../errors/DataTriggerInvalidResultError');

class DataTrigger {
  /**
   * @param {string} dataContractId
   * @param {string} documentType
   * @param {number} documentAction
   * @param {
   * function(Document, DataTriggerExecutionContext, string):DataTriggerExecutionResult
   * } trigger
   * @param {string} topLevelIdentity
   */
  constructor(dataContractId, documentType, documentAction, trigger, topLevelIdentity) {
    this.dataContractId = dataContractId;
    this.documentType = documentType;
    this.documentAction = documentAction;
    this.trigger = trigger;
    this.topLevelIdentity = topLevelIdentity;
  }

  /**
   * Check this trigger is matching for specified data
   *
   * @param {string} dataContractId
   * @param {string} documentType
   * @param {number} documentAction
   *
   * @return {boolean}
   */
  isMatchingTriggerForData(dataContractId, documentType, documentAction) {
    return this.dataContractId === dataContractId
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
      result = await this.trigger(document, context, this.topLevelIdentity);
    } catch (e) {
      result.addError(
        new DataTriggerExecutionError(
          this, context.getDataContract(), context.getUserId(), e,
        ),
      );
    }

    if (!(result instanceof DataTriggerExecutionResult)) {
      result = new DataTriggerExecutionResult();
      result.addError(
        new DataTriggerInvalidResultError(
          this, context.getDataContract(), context.getUserId(),
        ),
      );
    }

    return result;
  }
}

module.exports = DataTrigger;

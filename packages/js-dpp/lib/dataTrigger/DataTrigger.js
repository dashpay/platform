const DataTriggerExecutionResult = require('./DataTriggerExecutionResult');
const DataTriggerExecutionError = require('../errors/DataTriggerExecutionError');
const DataTriggerInvalidResultError = require('../errors/DataTriggerInvalidResultError');

class DataTrigger {
  /**
   * @param {Buffer|Identifier} dataContractId
   * @param {string} documentType
   * @param {number} transitionAction
   * @param {
   * function(DocumentCreateTransition[]
   *    |DocumentReplaceTransition[]
   *    |DocumentDeleteTransition[], DataTriggerExecutionContext, string):DataTriggerExecutionResult
   * } trigger
   * @param {Buffer|Identifier} topLevelIdentity
   */
  constructor(dataContractId, documentType, transitionAction, trigger, topLevelIdentity) {
    this.dataContractId = dataContractId;
    this.documentType = documentType;
    this.transitionAction = transitionAction;
    this.trigger = trigger;
    this.topLevelIdentity = topLevelIdentity;
  }

  /**
   * Check this trigger is matching for specified data
   *
   * @param {string} dataContractId
   * @param {string} documentType
   * @param {number} transitionAction
   *
   * @return {boolean}
   */
  isMatchingTriggerForData(dataContractId, documentType, transitionAction) {
    return this.dataContractId.equals(dataContractId)
      && this.documentType === documentType
      && this.transitionAction === transitionAction;
  }

  /**
   * Execute data trigger
   *
   * @param {DocumentCreateTransition[]
   *        |DocumentReplaceTransition[]
   *        |DocumentDeleteTransition[]} documentTransition
   * @param {DataTriggerExecutionContext} context
   *
   * @returns {Promise<DataTriggerExecutionResult>}
   */
  async execute(documentTransition, context) {
    let result = new DataTriggerExecutionResult();

    try {
      result = await this.trigger(documentTransition, context, this.topLevelIdentity);
    } catch (e) {
      result.addError(
        new DataTriggerExecutionError(
          this,
          context.getDataContract(),
          context.getOwnerId(),
          e,
        ),
      );
    }

    if (!(result instanceof DataTriggerExecutionResult)) {
      result = new DataTriggerExecutionResult();
      result.addError(
        new DataTriggerInvalidResultError(
          this,
          context.getDataContract(),
          context.getOwnerId(),
        ),
      );
    }

    return result;
  }
}

module.exports = DataTrigger;

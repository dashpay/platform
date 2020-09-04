const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

/**
 * Data trigger for domain update process
 *
 * @param {DocumentReplaceTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function rejectUpdateDataTrigger(documentTransition, context) {
  const result = new DataTriggerExecutionResult();

  result.addError(
    new DataTriggerConditionError(
      documentTransition,
      context.getDataContract(),
      context.getOwnerId(),
      'Update action is not allowed',
    ),
  );

  return result;
}

module.exports = rejectUpdateDataTrigger;

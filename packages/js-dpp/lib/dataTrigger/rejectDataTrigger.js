const DataTriggerExecutionResult = require('./DataTriggerExecutionResult');
const DataTriggerConditionError = require('../errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');

/**
 * Data trigger for domain deletion process
 *
 * @param {DocumentDeleteTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function rejectDataTrigger(documentTransition, context) {
  const result = new DataTriggerExecutionResult();

  result.addError(
    new DataTriggerConditionError(
      documentTransition,
      context.getDataContract(),
      context.getOwnerId(),
      'Action is not allowed',
    ),
  );

  return result;
}

module.exports = rejectDataTrigger;

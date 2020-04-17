const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

/**
 * Data trigger for domain deletion process
 *
 * @param {Document} document
 * @param {DataTriggerExecutionContext} context
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function deleteDomainDataTrigger(document, context) {
  const result = new DataTriggerExecutionResult();

  result.addError(
    new DataTriggerConditionError(
      document,
      context.getDataContract(),
      context.getOwnerId(),
      'Delete action is not allowed',
    ),
  );

  return result;
}

module.exports = deleteDomainDataTrigger;

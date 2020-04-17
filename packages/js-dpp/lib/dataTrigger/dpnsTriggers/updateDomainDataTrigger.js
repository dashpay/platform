const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

/**
 * Data trigger for domain update process
 *
 * @param {Document} document
 * @param {DataTriggerExecutionContext} context
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function updateDomainDataTrigger(document, context) {
  const result = new DataTriggerExecutionResult();

  result.addError(
    new DataTriggerConditionError(
      document,
      context.getDataContract(),
      context.getOwnerId(),
      'Update action is not allowed',
    ),
  );

  return result;
}

module.exports = updateDomainDataTrigger;

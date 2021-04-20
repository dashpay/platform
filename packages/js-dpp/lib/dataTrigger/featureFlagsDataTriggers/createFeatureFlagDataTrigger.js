const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');
const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');

async function createFeatureFlagDataTrigger(documentTransition, context, topLevelIdentity) {
  const result = new DataTriggerExecutionResult();

  if (!context.getOwnerId().equals(topLevelIdentity)) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'This identity can\'t activate selected feature flag',
      ),
    );
  }

  return result;
}

module.exports = createFeatureFlagDataTrigger;

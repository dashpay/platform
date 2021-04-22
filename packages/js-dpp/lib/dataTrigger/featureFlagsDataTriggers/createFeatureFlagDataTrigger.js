const Long = require('long');

const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');
const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');

async function createFeatureFlagDataTrigger(documentTransition, context, topLevelIdentity) {
  const result = new DataTriggerExecutionResult();

  const stateRepository = context.getStateRepository();

  const { height: blockHeight } = await stateRepository.fetchLatestPlatformBlockHeader();

  if (Long.fromNumber(documentTransition.get('enableAtHeight')).lt(blockHeight)) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Feature flag cannot be enabled in the past',
      ),
    );

    return result;
  }

  if (!context.getOwnerId().equals(topLevelIdentity.getId())) {
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

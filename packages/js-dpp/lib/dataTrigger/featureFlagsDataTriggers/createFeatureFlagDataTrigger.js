const Long = require('long');

const DataTriggerConditionError = require('../../errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');
const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');

/**
 * @param {DocumentCreateTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 * @param {Identifier} topLevelIdentityId
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function createFeatureFlagDataTrigger(documentTransition, context, topLevelIdentityId) {
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

  if (!context.getOwnerId().equals(topLevelIdentityId)) {
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

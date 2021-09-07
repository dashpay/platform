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
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `Feature flag cannot be enabled in the past on block ${documentTransition.get('enableAtHeight')}. Current block height is ${blockHeight}`,
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);

    return result;
  }

  if (!context.getOwnerId().equals(topLevelIdentityId)) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      'This identity can\'t activate selected feature flag',
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  return result;
}

module.exports = createFeatureFlagDataTrigger;

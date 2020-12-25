const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/DataTriggerConditionError');

const BLOCKS_WINDOW_SIZE = 8;

/**
 * Data trigger for contract request creation process
 *
 * @param {DocumentCreateTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 *
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function createContactRequestDataTrigger(documentTransition, context) {
  const {
    coreHeightCreatedAt,
  } = documentTransition.getData();

  const result = new DataTriggerExecutionResult();

  if (coreHeightCreatedAt === undefined) {
    return result;
  }

  const stateRepository = context.getStateRepository();

  const latestPlatformBlockHeader = await stateRepository.fetchLatestPlatformBlockHeader();

  const { coreChainLockedHeight } = latestPlatformBlockHeader;

  const heightWindowStart = coreChainLockedHeight - BLOCKS_WINDOW_SIZE;
  const heightWindowEnd = coreChainLockedHeight + BLOCKS_WINDOW_SIZE;

  if (coreHeightCreatedAt < heightWindowStart || coreHeightCreatedAt > heightWindowEnd) {
    result.addError(
      new DataTriggerConditionError(
        documentTransition,
        context.getDataContract(),
        context.getOwnerId(),
        'Core height is out of block height window',
      ),
    );
  }

  return result;
}

module.exports = createContactRequestDataTrigger;

const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');
const DataTriggerConditionError = require('../../errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');

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
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `Core height ${coreHeightCreatedAt} is out of block height window from ${heightWindowStart} to ${heightWindowEnd}`,
    );

    error.setOwnerId(context.getOwnerId());
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  return result;
}

module.exports = createContactRequestDataTrigger;

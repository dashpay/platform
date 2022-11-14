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
  const result = new DataTriggerExecutionResult();
  const isDryRun = context.getStateTransitionExecutionContext().isDryRun();

  const {
    coreHeightCreatedAt,
    toUserId,
  } = documentTransition.getData();

  const stateRepository = context.getStateRepository();
  const ownerId = context.getOwnerId();

  if (!isDryRun) {
    if (ownerId.equals(toUserId)) {
      const error = new DataTriggerConditionError(
        context.getDataContract().getId().toBuffer(),
        documentTransition.getId().toBuffer(),
        `Identity ${toUserId.toString()} must not be equal to the owner`,
      );

      error.setOwnerId(ownerId);
      error.setDocumentTransition(documentTransition);

      result.addError(error);

      return result;
    }

    if (coreHeightCreatedAt !== undefined) {
      const coreChainLockedHeight = await stateRepository
        .fetchLatestPlatformCoreChainLockedHeight();

      const heightWindowStart = coreChainLockedHeight - BLOCKS_WINDOW_SIZE;
      const heightWindowEnd = coreChainLockedHeight + BLOCKS_WINDOW_SIZE;

      if (coreHeightCreatedAt < heightWindowStart || coreHeightCreatedAt > heightWindowEnd) {
        const error = new DataTriggerConditionError(
          context.getDataContract().getId().toBuffer(),
          documentTransition.getId().toBuffer(),
          `Core height ${coreHeightCreatedAt} is out of block height window from ${heightWindowStart} to ${heightWindowEnd}`,
        );

        error.setOwnerId(ownerId);
        error.setDocumentTransition(documentTransition);

        result.addError(error);

        return result;
      }
    }
  }

  // toUserId identity exists
  const identity = await stateRepository.fetchIdentity(
    toUserId,
    context.getStateTransitionExecutionContext(),
  );

  if (!isDryRun && identity === null) {
    const error = new DataTriggerConditionError(
      context.getDataContract()
        .getId()
        .toBuffer(),
      documentTransition.getId()
        .toBuffer(),
      `Identity ${toUserId.toString()} doesn't exist`,
    );

    error.setOwnerId(ownerId);
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  return result;
}

module.exports = createContactRequestDataTrigger;

const DataTriggerConditionError = require('../../errors/consensus/state/dataContract/dataTrigger/DataTriggerConditionError');
const DataTriggerExecutionResult = require('../DataTriggerExecutionResult');

const MAX_PERCENTAGE = 10000;

/**
 * @param {DocumentCreateTransition} documentTransition
 * @param {DataTriggerExecutionContext} context
 * @param {Identifier} topLevelIdentityId
 * @return {Promise<DataTriggerExecutionResult>}
 */
async function createMasternodeRewardSharesDataTrigger(
  documentTransition,
  context,
  topLevelIdentityId,
) {
  const {
    payToId,
    percentage,
  } = documentTransition.getData();

  const ownerId = context.getOwnerId();

  const result = new DataTriggerExecutionResult();

  if (percentage > MAX_PERCENTAGE) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `Percentage can not be more than ${MAX_PERCENTAGE}`,
    );

    error.setOwnerId(ownerId);
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  const identity = await context.getStateRepository().fetchIdentity(payToId);
  if (identity !== null) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `Identity ${payToId.toString()} already exists`,
    );

    error.setOwnerId(ownerId);
    error.setDocumentTransition(documentTransition);

    result.addError(error);
  }

  const smlStore = await context.getStateRepository().fetchSMLStore();
  const validMasternodesList = smlStore.getCurrentSML().getValidMasternodesList();
  const ownerIdInSml = !!validMasternodesList.find((smlEntry) => Buffer.compare(ownerId, Buffer.from(smlEntry.proRegTxHash, 'hex')) === 0);

  if (!ownerIdInSml) {
    const error = new DataTriggerConditionError(
      context.getDataContract().getId().toBuffer(),
      documentTransition.getId().toBuffer(),
      `Owner ID ${ownerId.toString()} is not in SML`,
    );

    error.setOwnerId(ownerId);
    error.setDocumentTransition(documentTransition);

    result.addError(error);
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

module.exports = createMasternodeRewardSharesDataTrigger;

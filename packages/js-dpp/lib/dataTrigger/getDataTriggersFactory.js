const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');
const featureFlagsSystemIds = require('@dashevo/feature-flags-contract/lib/systemIds');

const dpnsSystemIds = require('@dashevo/dpns-contract/lib/systemIds');
const dashpaySystemIds = require('@dashevo/dashpay-contract/lib/systemIds');
const masternodeRewardSharesSystemIds = require('@dashevo/masternode-reward-shares-contract/lib/systemIds');

const Identifier = require('../identifier/Identifier');

const AbstractDocumentTransition = require('../document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');

const DataTrigger = require('./DataTrigger');

const rejectDataTrigger = require('./rejectDataTrigger');
const createDomainDataTrigger = require('./dpnsTriggers/createDomainDataTrigger');
const createContactRequestDataTrigger = require('./dashpayDataTriggers/createContactRequestDataTrigger');
const createFeatureFlagDataTrigger = require('./featureFlagsDataTriggers/createFeatureFlagDataTrigger');
const createMasternodeRewardSharesDataTrigger = require('./rewardShareDataTriggers/createMasternodeRewardSharesDataTrigger');

/**
 * Get respective data triggers (factory)
 *
 * @return {getDataTriggers}
 */
function getDataTriggersFactory() {
  const dpnsDataContractId = Identifier.from(dpnsSystemIds.contractId);
  const dpnsOwnerId = Identifier.from(dpnsSystemIds.ownerId);

  const dashPayDataContractId = Identifier.from(dashpaySystemIds.contractId);

  const featureFlagsDataContractId = Identifier.from(featureFlagsSystemIds.contractId);
  const featureFlagsOwnerId = Identifier.from(
    featureFlagsSystemIds.ownerId,
  );

  const masternodeRewardSharesContractId = Identifier.from(
    masternodeRewardSharesSystemIds.contractId,
  );

  const dataTriggers = [
    new DataTrigger(
      dpnsDataContractId,
      'domain',
      AbstractDocumentTransition.ACTIONS.CREATE,
      createDomainDataTrigger,
      dpnsOwnerId,
    ),
    new DataTrigger(
      dpnsDataContractId,
      'domain',
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      dpnsDataContractId,
      'domain',
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      dpnsDataContractId,
      'preorder',
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      dpnsDataContractId,
      'preorder',
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      dashPayDataContractId,
      'contactRequest',
      AbstractDocumentTransition.ACTIONS.CREATE,
      createContactRequestDataTrigger,
    ),
    new DataTrigger(
      dashPayDataContractId,
      'contactRequest',
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      dashPayDataContractId,
      'contactRequest',
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      featureFlagsDataContractId,
      featureFlagTypes.UPDATE_CONSENSUS_PARAMS,
      AbstractDocumentTransition.ACTIONS.CREATE,
      createFeatureFlagDataTrigger,
      featureFlagsOwnerId,
    ),
    new DataTrigger(
      featureFlagsDataContractId,
      featureFlagTypes.UPDATE_CONSENSUS_PARAMS,
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      featureFlagsDataContractId,
      featureFlagTypes.UPDATE_CONSENSUS_PARAMS,
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      featureFlagsDataContractId,
      featureFlagTypes.FIX_CUMULATIVE_FEES,
      AbstractDocumentTransition.ACTIONS.CREATE,
      createFeatureFlagDataTrigger,
      featureFlagsOwnerId,
    ),
    new DataTrigger(
      featureFlagsDataContractId,
      featureFlagTypes.FIX_CUMULATIVE_FEES,
      AbstractDocumentTransition.ACTIONS.REPLACE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      featureFlagsDataContractId,
      featureFlagTypes.FIX_CUMULATIVE_FEES,
      AbstractDocumentTransition.ACTIONS.DELETE,
      rejectDataTrigger,
    ),
    new DataTrigger(
      masternodeRewardSharesContractId,
      'rewardShare',
      AbstractDocumentTransition.ACTIONS.CREATE,
      createMasternodeRewardSharesDataTrigger,
    ),
    new DataTrigger(
      masternodeRewardSharesContractId,
      'rewardShare',
      AbstractDocumentTransition.ACTIONS.REPLACE,
      createMasternodeRewardSharesDataTrigger,
    ),
  ];

  /**
   * Get respective data triggers
   *
   * @typedef getDataTriggers
   *
   * @param {Identifier|Buffer} dataContractId
   * @param {string} documentType
   * @param {number} transitionAction
   *
   * @returns {DataTrigger[]}
   */
  function getDataTriggers(dataContractId, documentType, transitionAction) {
    return dataTriggers.filter(
      (dataTrigger) => dataTrigger.isMatchingTriggerForData(
        dataContractId,
        documentType,
        transitionAction,
      ),
    );
  }

  return getDataTriggers;
}

module.exports = getDataTriggersFactory;

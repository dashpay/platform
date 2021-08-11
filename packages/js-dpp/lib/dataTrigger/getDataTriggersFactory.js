const featureFlagTypes = require('@dashevo/feature-flags-contract/lib/featureFlagTypes');

const Identifier = require('../identifier/Identifier');

const AbstractDocumentTransition = require('../document/stateTransition/DocumentsBatchTransition/documentTransition/AbstractDocumentTransition');

const DataTrigger = require('./DataTrigger');

const rejectDataTrigger = require('./rejectDataTrigger');
const createDomainDataTrigger = require('./dpnsTriggers/createDomainDataTrigger');
const createContactRequestDataTrigger = require('./dashpayDataTriggers/createContactRequestDataTrigger');
const createFeatureFlagDataTrigger = require('./featureFlagsDataTriggers/createFeatureFlagDataTrigger');

/**
 * Get respective data triggers (factory)
 *
 * @return {getDataTriggers}
 */
function getDataTriggersFactory() {
  let dpnsDataContractId = Buffer.alloc(0);
  if (process.env.DPNS_CONTRACT_ID) {
    dpnsDataContractId = Identifier.from(process.env.DPNS_CONTRACT_ID);
  }

  let dpnsTopLevelIdentityId = Buffer.alloc(0);
  if (process.env.DPNS_TOP_LEVEL_IDENTITY) {
    dpnsTopLevelIdentityId = Identifier.from(process.env.DPNS_TOP_LEVEL_IDENTITY);
  }

  let dashPayDataContractId = Buffer.alloc(0);
  if (process.env.DASHPAY_CONTRACT_ID) {
    dashPayDataContractId = Identifier.from(process.env.DASHPAY_CONTRACT_ID);
  }

  let featureFlagsDataContractId = Buffer.alloc(0);
  if (process.env.FEATURE_FLAGS_CONTRACT_ID) {
    featureFlagsDataContractId = Identifier.from(process.env.FEATURE_FLAGS_CONTRACT_ID);
  }

  let featureFlagsTopLevelIdentityId = Buffer.alloc(0);
  if (process.env.FEATURE_FLAGS_TOP_LEVEL_IDENTITY) {
    featureFlagsTopLevelIdentityId = Identifier.from(
      process.env.FEATURE_FLAGS_TOP_LEVEL_IDENTITY,
    );
  }

  const dataTriggers = [
    new DataTrigger(
      dpnsDataContractId,
      'domain',
      AbstractDocumentTransition.ACTIONS.CREATE,
      createDomainDataTrigger,
      dpnsTopLevelIdentityId,
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
      featureFlagsTopLevelIdentityId,
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
      featureFlagsTopLevelIdentityId,
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
